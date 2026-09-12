//! Guest ELF64 PT_LOAD loader into user TTBR0 (Track A / A3 / ADR-023).
//!
//! The kernel parses an embedded ELF (the A2 `libctos` hello) and maps
//! each `PT_LOAD` into the user map-window, then `ERET`s to `e_entry`.
//! That is **not** the A2 host extract + memcpy onto `EL0_PAGE`.
//! A4 / ADR-024 reuses this loader as the way a standing **task**
//! appears (`run_hello_as_task`). Not a Linux ELF ABI. Not `PT_INTERP`.
//! Not glibc. Not app hosting.

use core::fmt::Write;
use core::hint::black_box;

use crate::el0;
use crate::exception;
use crate::frame;
use crate::paging;
use crate::syscall;
use crate::uart;

include!(concat!(env!("OUT_DIR"), "/hello_libctos_meta.rs"));

const HELLO_ELF: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/hello-libctos.elf"));

const _: () = assert!(HELLO_ELF_LEN > 64);
const _: () = assert!(HELLO_ELF.len() == HELLO_ELF_LEN);
const _: () = assert!(HELLO_ELF[0] == 0x7f && HELLO_ELF[1] == b'E');
const _: () = assert!(HELLO_LOAD_VA == 0x8000_2000);
const _: () = assert!(HELLO_LEN > 0);

const EI_CLASS: usize = 4;
const EI_DATA: usize = 5;
const EI_VERSION: usize = 6;
const ELFCLASS64: u8 = 2;
const ELFDATA2LSB: u8 = 1;
const EV_CURRENT: u8 = 1;
const ET_EXEC: u16 = 2;
const EM_AARCH64: u16 = 183;
const PT_LOAD: u32 = 1;
const PT_INTERP: u32 = 3;
const PF_X: u32 = 1;
const PF_W: u32 = 2;
const EHSIZE: usize = 64;
const PHDR_SIZE: usize = 56;
const PAGE: u64 = 4096;
const MAX_SEGS: usize = 8;
const MAX_PAGES: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ParseError {
    Truncated,
    NotElf,
    NotElf64Le,
    NotExec,
    NotAarch64,
    BadPhdrs,
    Interp,
    WxSegment,
    BadLoad,
    TooManyLoads,
    Empty,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Perm {
    Exec,
    Rw,
    Ro,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LoadSeg {
    pub offset: u64,
    pub vaddr: u64,
    pub filesz: u64,
    pub memsz: u64,
    pub flags: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Image {
    pub entry: u64,
    pub segs: [LoadSeg; MAX_SEGS],
    pub nseg: usize,
}

#[derive(Clone, Copy)]
struct MappedPage {
    va: u64,
    pa: u64,
    perm: Perm,
}

fn read_u16(buf: &[u8], off: usize) -> Result<u16, ParseError> {
    let b = buf.get(off..off + 2).ok_or(ParseError::Truncated)?;
    Ok(u16::from_le_bytes([b[0], b[1]]))
}

fn read_u32(buf: &[u8], off: usize) -> Result<u32, ParseError> {
    let b = buf.get(off..off + 4).ok_or(ParseError::Truncated)?;
    Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

fn read_u64(buf: &[u8], off: usize) -> Result<u64, ParseError> {
    let b = buf.get(off..off + 8).ok_or(ParseError::Truncated)?;
    Ok(u64::from_le_bytes([
        b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
    ]))
}

fn perm_of(flags: u32) -> Result<Perm, ParseError> {
    let x = flags & PF_X != 0;
    let w = flags & PF_W != 0;
    if x && w {
        return Err(ParseError::WxSegment);
    }
    if x {
        Ok(Perm::Exec)
    } else if w {
        Ok(Perm::Rw)
    } else {
        Ok(Perm::Ro)
    }
}

/// Page-granular union. `Ro`+`Exec` is the usual small-image case
/// (`.text` RX and `.rodata` R on one 4 KiB page). `Exec`+`Rw` is W+X.
fn combine_perm(a: Perm, b: Perm) -> Result<Perm, ParseError> {
    match (a, b) {
        (Perm::Ro, Perm::Ro) => Ok(Perm::Ro),
        (Perm::Rw, Perm::Rw) | (Perm::Rw, Perm::Ro) | (Perm::Ro, Perm::Rw) => Ok(Perm::Rw),
        (Perm::Exec, Perm::Exec) | (Perm::Exec, Perm::Ro) | (Perm::Ro, Perm::Exec) => {
            Ok(Perm::Exec)
        }
        (Perm::Exec, Perm::Rw) | (Perm::Rw, Perm::Exec) => Err(ParseError::WxSegment),
    }
}

/// Parse a freestanding ELF64 LE AArch64 `ET_EXEC`. `PT_LOAD` only.
/// Rejects `PT_INTERP` and W+X segments. Not a Linux ABI.
pub(crate) fn parse_elf64(elf: &[u8]) -> Result<Image, ParseError> {
    if elf.len() < EHSIZE {
        return Err(ParseError::Truncated);
    }
    if &elf[0..4] != b"\x7fELF" {
        return Err(ParseError::NotElf);
    }
    if elf[EI_CLASS] != ELFCLASS64 || elf[EI_DATA] != ELFDATA2LSB || elf[EI_VERSION] != EV_CURRENT {
        return Err(ParseError::NotElf64Le);
    }
    if read_u16(elf, 16)? != ET_EXEC {
        return Err(ParseError::NotExec);
    }
    if read_u16(elf, 18)? != EM_AARCH64 {
        return Err(ParseError::NotAarch64);
    }
    if read_u32(elf, 20)? != 1 {
        return Err(ParseError::NotElf64Le);
    }
    let entry = read_u64(elf, 24)?;
    let phoff = read_u64(elf, 32)? as usize;
    let ehsize = read_u16(elf, 52)? as usize;
    let phentsize = read_u16(elf, 54)? as usize;
    let phnum = read_u16(elf, 56)? as usize;
    if ehsize != EHSIZE || phentsize < PHDR_SIZE || phoff == 0 || phnum == 0 || phnum > 16 {
        return Err(ParseError::BadPhdrs);
    }

    let mut segs = [LoadSeg {
        offset: 0,
        vaddr: 0,
        filesz: 0,
        memsz: 0,
        flags: 0,
    }; MAX_SEGS];
    let mut nseg = 0usize;

    for i in 0..phnum {
        let off = phoff
            .checked_add(i.checked_mul(phentsize).ok_or(ParseError::BadPhdrs)?)
            .ok_or(ParseError::BadPhdrs)?;
        if off.checked_add(PHDR_SIZE).ok_or(ParseError::Truncated)? > elf.len() {
            return Err(ParseError::Truncated);
        }
        let p_type = read_u32(elf, off)?;
        if p_type == PT_INTERP {
            return Err(ParseError::Interp);
        }
        if p_type != PT_LOAD {
            continue;
        }
        let p_flags = read_u32(elf, off + 4)?;
        let p_offset = read_u64(elf, off + 8)?;
        let p_vaddr = read_u64(elf, off + 16)?;
        let p_filesz = read_u64(elf, off + 32)?;
        let p_memsz = read_u64(elf, off + 40)?;
        if p_memsz == 0 {
            continue;
        }
        if p_filesz > p_memsz {
            return Err(ParseError::BadLoad);
        }
        if p_vaddr.checked_add(p_memsz).is_none() {
            return Err(ParseError::BadLoad);
        }
        if (p_vaddr & 0xfff) != (p_offset & 0xfff) {
            return Err(ParseError::BadLoad);
        }
        let file_end = p_offset.checked_add(p_filesz).ok_or(ParseError::BadLoad)?;
        if file_end > elf.len() as u64 {
            return Err(ParseError::BadLoad);
        }
        if !paging::window_range_ok(p_vaddr, p_memsz) {
            return Err(ParseError::BadLoad);
        }
        let _ = perm_of(p_flags)?;
        if nseg >= MAX_SEGS {
            return Err(ParseError::TooManyLoads);
        }
        segs[nseg] = LoadSeg {
            offset: p_offset,
            vaddr: p_vaddr,
            filesz: p_filesz,
            memsz: p_memsz,
            flags: p_flags,
        };
        nseg += 1;
    }
    if nseg == 0 {
        return Err(ParseError::Empty);
    }
    if !paging::window_range_ok(entry, 4) {
        return Err(ParseError::BadLoad);
    }
    Ok(Image { entry, segs, nseg })
}

fn sync_range(ptr: *const u8, len: usize) {
    let mut x = ptr as u64;
    let end = x + len as u64;
    while x < end {
        unsafe {
            core::arch::asm!(
                "dc cvau, {x}",
                "ic ivau, {x}",
                x = in(reg) x,
                options(nostack, preserves_flags),
            );
        }
        x += 64;
    }
    unsafe {
        core::arch::asm!("dsb ish", "isb", options(nostack, preserves_flags));
    }
}

fn find_page(pages: &[Option<MappedPage>; MAX_PAGES], va: u64) -> Option<usize> {
    let page_va = va & !0xfff;
    pages
        .iter()
        .position(|p| p.is_some_and(|m| m.va == page_va))
}

fn map_perm(va: u64, pa: u64, perm: Perm) -> bool {
    match perm {
        Perm::Exec => paging::map_el0_exec(va, pa),
        Perm::Rw => paging::map_el0_rw(va, pa),
        Perm::Ro => paging::map_el0_ro(va, pa),
    }
}

fn write_pa(pa: u64, off: u64, byte: u8) {
    unsafe {
        core::ptr::write_volatile((pa as *mut u8).add(off as usize), byte);
    }
}

fn teardown(pages: &[Option<MappedPage>; MAX_PAGES], stack: Option<(u64, u64)>) {
    for slot in pages {
        if let Some(m) = slot {
            let _ = paging::unmap_page(m.va);
            frame::free(m.pa);
        }
    }
    if let Some((va, pa)) = stack {
        let _ = paging::unmap_page(va);
        frame::free(pa);
    }
}

fn plan_pages(img: &Image) -> Result<([Option<(u64, Perm)>; MAX_PAGES], usize), ParseError> {
    let mut plan: [Option<(u64, Perm)>; MAX_PAGES] = [None; MAX_PAGES];
    let mut n = 0usize;
    for i in 0..img.nseg {
        let seg = img.segs[i];
        let perm = perm_of(seg.flags)?;
        let start = seg.vaddr & !0xfff;
        let end = (seg.vaddr + seg.memsz + PAGE - 1) & !0xfff;
        let mut va = start;
        while va < end {
            if let Some(slot) = plan
                .iter_mut()
                .find(|p| p.as_ref().is_some_and(|(pva, _)| *pva == va))
            {
                let (pva, old) = slot.unwrap();
                *slot = Some((pva, combine_perm(old, perm)?));
            } else {
                if n >= MAX_PAGES {
                    return Err(ParseError::TooManyLoads);
                }
                plan[n] = Some((va, perm));
                n += 1;
            }
            va += PAGE;
        }
    }
    Ok((plan, n))
}

fn load_image(
    elf: &[u8],
    img: &Image,
) -> Result<(u64, [Option<MappedPage>; MAX_PAGES], Option<(u64, u64)>), ()> {
    let mut pages: [Option<MappedPage>; MAX_PAGES] = [None; MAX_PAGES];
    let Ok((plan, nplan)) = plan_pages(img) else {
        return Err(());
    };
    for i in 0..nplan {
        let (page_va, perm) = plan[i].unwrap();
        let Some(pa) = frame::alloc() else {
            teardown(&pages, None);
            return Err(());
        };
        unsafe {
            core::ptr::write_bytes(pa as *mut u8, 0, PAGE as usize);
        }
        if !map_perm(page_va, pa, perm) {
            frame::free(pa);
            teardown(&pages, None);
            return Err(());
        }
        pages[i] = Some(MappedPage {
            va: page_va,
            pa,
            perm,
        });
    }

    for i in 0..img.nseg {
        let seg = img.segs[i];
        for j in 0..seg.filesz {
            let dest = seg.vaddr + j;
            let Some(idx) = find_page(&pages, dest) else {
                teardown(&pages, None);
                return Err(());
            };
            let m = pages[idx].unwrap();
            let src = (seg.offset + j) as usize;
            write_pa(m.pa, dest & 0xfff, elf[src]);
        }
        for j in seg.filesz..seg.memsz {
            let dest = seg.vaddr + j;
            let Some(idx) = find_page(&pages, dest) else {
                teardown(&pages, None);
                return Err(());
            };
            let m = pages[idx].unwrap();
            write_pa(m.pa, dest & 0xfff, 0);
        }
    }

    let entry_page = img.entry & !0xfff;
    let Some(idx) = find_page(&pages, entry_page) else {
        teardown(&pages, None);
        return Err(());
    };
    if pages[idx].unwrap().perm != Perm::Exec {
        teardown(&pages, None);
        return Err(());
    }

    for slot in &pages {
        if let Some(m) = slot {
            if m.perm == Perm::Exec {
                sync_range(m.pa as *const u8, PAGE as usize);
                sync_range(m.va as *const u8, PAGE as usize);
            }
        }
    }

    let stack_va = paging::LOADER_STACK_VA;
    if find_page(&pages, stack_va).is_some() {
        teardown(&pages, None);
        return Err(());
    }
    let Some(stack_pa) = frame::alloc() else {
        teardown(&pages, None);
        return Err(());
    };
    unsafe {
        core::ptr::write_bytes(stack_pa as *mut u8, 0, PAGE as usize);
    }
    if !paging::map_el0_rw(stack_va, stack_pa) {
        frame::free(stack_pa);
        teardown(&pages, None);
        return Err(());
    }

    Ok((img.entry, pages, Some((stack_va, stack_pa))))
}

fn run_loaded() -> bool {
    if el0::is_active() || !paging::user_map_ready() {
        return false;
    }
    let Ok(img) = parse_elf64(HELLO_ELF) else {
        return false;
    };
    syscall::reset_probe_flags();
    let Ok((entry, pages, stack)) = load_image(HELLO_ELF, &img) else {
        return false;
    };
    let Some((stack_va, stack_pa)) = stack else {
        teardown(&pages, None);
        return false;
    };
    let user_sp = stack_va + PAGE;
    el0::install_standing(entry, user_sp, paging::user_ttbr0());
    unsafe {
        exception::eret_to_el0(black_box(entry), 0, user_sp);
    }
    teardown(&pages, Some((stack_va, stack_pa)));
    if el0::is_active() {
        el0::clear_active();
        return false;
    }
    syscall::yield_seen()
        && syscall::uart_seen()
        && syscall::exit_seen()
        && syscall::last_uart_write() == 12
        && syscall::last_exit_status() == 0
        && syscall::yield_count() >= 1
}

/// A4 / ADR-024: same A3 map, but standing **task** until `SYS_EXIT`.
/// `is_active()` is true for the lifetime; leftover active is Failed.
pub(crate) fn run_hello_as_task() -> bool {
    if el0::is_active() || !paging::user_map_ready() {
        return false;
    }
    let Ok(img) = parse_elf64(HELLO_ELF) else {
        return false;
    };
    syscall::reset_probe_flags();
    el0::reset_task_flags();
    let Ok((entry, pages, stack)) = load_image(HELLO_ELF, &img) else {
        return false;
    };
    let Some((stack_va, stack_pa)) = stack else {
        teardown(&pages, None);
        return false;
    };
    let user_sp = stack_va + PAGE;
    el0::install_task(entry, user_sp, paging::user_ttbr0());
    if !el0::is_active() || !el0::is_task() {
        teardown(&pages, Some((stack_va, stack_pa)));
        el0::clear_active();
        return false;
    }
    unsafe {
        exception::eret_to_el0(black_box(entry), 0, user_sp);
    }
    teardown(&pages, Some((stack_va, stack_pa)));
    if el0::is_active() {
        el0::clear_active();
        return false;
    }
    el0::task_active_seen()
        && el0::task_exit_seen()
        && syscall::yield_seen()
        && syscall::uart_seen()
        && syscall::exit_seen()
        && syscall::last_uart_write() == 12
        && syscall::last_exit_status() == 0
        && syscall::yield_count() >= 1
}

/// Serial proof: guest parsed ELF, mapped PT_LOAD, ERET to e_entry.
#[allow(dead_code)] // hello kernel only; cargo test uses the cases below.
pub fn observe_probe() -> bool {
    if !paging::mmu_enabled() || !paging::user_map_ready() {
        return false;
    }
    let Ok(img) = parse_elf64(HELLO_ELF) else {
        return false;
    };
    if !run_loaded() {
        return false;
    }
    let mut w = uart::raw();
    let _ = writeln!(w, "loader: mapped segs={} entry={:#x}", img.nseg, img.entry);
    let _ = writeln!(w, "loader: ok");
    true
}

#[cfg(test)]
fn put_u16(buf: &mut [u8], off: usize, v: u16) {
    buf[off..off + 2].copy_from_slice(&v.to_le_bytes());
}

#[cfg(test)]
fn put_u32(buf: &mut [u8], off: usize, v: u32) {
    buf[off..off + 4].copy_from_slice(&v.to_le_bytes());
}

#[cfg(test)]
fn put_u64(buf: &mut [u8], off: usize, v: u64) {
    buf[off..off + 8].copy_from_slice(&v.to_le_bytes());
}

#[cfg(test)]
fn write_ehdr(buf: &mut [u8], entry: u64, phoff: u64, phnum: u16) {
    buf[0..4].copy_from_slice(b"\x7fELF");
    buf[EI_CLASS] = ELFCLASS64;
    buf[EI_DATA] = ELFDATA2LSB;
    buf[EI_VERSION] = EV_CURRENT;
    put_u16(buf, 16, ET_EXEC);
    put_u16(buf, 18, EM_AARCH64);
    put_u32(buf, 20, 1);
    put_u64(buf, 24, entry);
    put_u64(buf, 32, phoff);
    put_u16(buf, 52, EHSIZE as u16);
    put_u16(buf, 54, PHDR_SIZE as u16);
    put_u16(buf, 56, phnum);
}

#[cfg(test)]
fn write_phdr(
    buf: &mut [u8],
    off: usize,
    p_type: u32,
    flags: u32,
    offset: u64,
    vaddr: u64,
    filesz: u64,
    memsz: u64,
) {
    put_u32(buf, off, p_type);
    put_u32(buf, off + 4, flags);
    put_u64(buf, off + 8, offset);
    put_u64(buf, off + 16, vaddr);
    put_u64(buf, off + 32, filesz);
    put_u64(buf, off + 40, memsz);
}

#[cfg(test)]
#[test_case]
fn loader_elf_is_not_the_a2_flat_image() {
    assert_eq!(&HELLO_ELF[0..4], b"\x7fELF");
    assert_ne!(
        &HELLO_ELF[0..4],
        &[0; 4],
        "A3 embed must be the ELF, not zeros"
    );
    let img = parse_elf64(HELLO_ELF).expect("hello ELF must parse");
    assert_eq!(img.entry, paging::EL0_PAGE);
    assert!(img.nseg >= 1);
    assert!(img.segs[..img.nseg]
        .iter()
        .any(|s| s.vaddr == paging::EL0_PAGE));
}

#[cfg(test)]
#[test_case]
fn loader_rejects_pt_interp() {
    let mut elf = [0u8; 128];
    write_ehdr(&mut elf, paging::EL0_PAGE, 64, 1);
    write_phdr(&mut elf, 64, PT_INTERP, 4, 0, 0, 0, 0);
    assert_eq!(parse_elf64(&elf), Err(ParseError::Interp));
}

#[cfg(test)]
#[test_case]
fn loader_unions_rx_and_ro_on_hello_page() {
    let img = parse_elf64(HELLO_ELF).expect("hello ELF must parse");
    let (plan, n) = plan_pages(&img).expect("RX text + R rodata may share a page");
    assert!(n >= 2, "header page + payload page");
    assert!(
        plan[..n]
            .iter()
            .any(|p| p.is_some_and(|(va, perm)| va == paging::EL0_PAGE && perm == Perm::Exec)),
        "payload page must stay executable after Ro union"
    );
}

#[cfg(test)]
#[test_case]
fn loader_rejects_wx_via_overlapping_pages() {
    let mut elf = [0u8; 192];
    write_ehdr(&mut elf, paging::EL0_PAGE, 64, 2);
    write_phdr(&mut elf, 64, PT_LOAD, PF_X, 0, paging::EL0_PAGE, 4, 4);
    write_phdr(
        &mut elf,
        64 + PHDR_SIZE,
        PT_LOAD,
        PF_W,
        0x80,
        paging::EL0_PAGE + 0x80,
        4,
        4,
    );
    let img = parse_elf64(&elf).expect("each segment is not W+X alone");
    assert_eq!(plan_pages(&img).err(), Some(ParseError::WxSegment));
}

#[cfg(test)]
#[test_case]
fn loader_rejects_wx_pt_load() {
    let mut elf = [0u8; 128];
    write_ehdr(&mut elf, paging::EL0_PAGE, 64, 1);
    write_phdr(
        &mut elf,
        64,
        PT_LOAD,
        PF_X | PF_W,
        0,
        paging::EL0_PAGE,
        4,
        4,
    );
    elf[0..4].copy_from_slice(b"\x7fELF");
    assert_eq!(parse_elf64(&elf), Err(ParseError::WxSegment));
}

#[cfg(test)]
#[test_case]
fn loader_rejects_truncated() {
    assert_eq!(parse_elf64(b"\x7fELF"), Err(ParseError::Truncated));
    assert_eq!(
        parse_elf64(b"notelf............notelf............notelf............notelf...."),
        Err(ParseError::NotElf)
    );
}

#[cfg(test)]
#[test_case]
fn loader_maps_hello_and_erets() {
    assert!(paging::mmu_enabled());
    assert!(
        run_loaded(),
        "guest ELF loader must map PT_LOAD, ERET, and see libctos hello"
    );
    assert!(!el0::is_active());
    assert_eq!(syscall::last_uart_write(), 12);
    assert_eq!(syscall::last_exit_status(), 0);
}
