//! Minimal EL0 SVC ABI (Track A / A1 / ADR-021).
//!
//! Public numbers start at 16 so they do not collide with the ADR-013
//! probe immediates (`SVC #0` first mile, `#1` standing, `#2` restore).
//! A6 adds 19–23 (thin VFS / memfs, ADR-027).
//! Not Linux. Not POSIX. Not app hosting.

use core::fmt::Write;
use core::hint::black_box;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::el0;
use crate::exception::{self, ExceptionContext};
use crate::frame;
use crate::paging;
use crate::uart;
use crate::vfs;

/// `SVC #0` first-mile return (ADR-013). Not a public ABI number.
pub const SVC_PROBE_RETURN: u64 = 0;
/// `SVC #1` standing announce (ADR-013). Not a public ABI number.
#[allow(dead_code)]
pub const SVC_PROBE_STANDING: u64 = 1;
/// `SVC #2` standing restore (ADR-013). Not a public ABI number.
#[allow(dead_code)]
pub const SVC_PROBE_RESTORE: u64 = 2;

/// Halt the EL0 trip and return to the EL1 caller. `x0` = status.
pub const SYS_EXIT: u64 = 16;
/// Write `x1` bytes from user pointer `x0` to the PL011. Returns count.
pub const SYS_UART_WRITE: u64 = 17;
/// Cooperative hint. This mile records the call and returns to EL0.
/// Does **not** run `sched::yield_now` (that path is EL1-only, ADR-010).
pub const SYS_YIELD: u64 = 18;
/// Create an empty memfs path and return a handle (ADR-027).
pub const SYS_FS_CREATE: u64 = 19;
/// Open an existing memfs path (ADR-027).
pub const SYS_FS_OPEN: u64 = 20;
/// Read from a memfs handle into a user buffer (ADR-027).
pub const SYS_FS_READ: u64 = 21;
/// Write a user buffer into a memfs handle (ADR-027).
pub const SYS_FS_WRITE: u64 = 22;
/// Close a memfs handle (ADR-027).
pub const SYS_FS_CLOSE: u64 = 23;

/// Hard cap on `SYS_UART_WRITE`. Longer lengths return 0.
pub const UART_WRITE_MAX: u64 = 64;
/// Hard cap on `SYS_FS_READ` / `SYS_FS_WRITE` copies.
pub const FS_IO_MAX: u64 = 64;
/// Reject value for `SYS_FS_READ` / `SYS_FS_CLOSE` (0 is a valid empty read).
pub const FS_ERR: u64 = u64::MAX;

/// User buffer the ABI probe writes via `SYS_UART_WRITE`.
pub const USER_UART_MSG: &[u8] = b"svc: user-hi\n";

const SVC16_A64: u32 = 0xD4000001 | ((SYS_EXIT as u32) << 5);
const SVC17_A64: u32 = 0xD4000001 | ((SYS_UART_WRITE as u32) << 5);
const SVC18_A64: u32 = 0xD4000001 | ((SYS_YIELD as u32) << 5);
const SVC19_A64: u32 = 0xD4000001 | ((SYS_FS_CREATE as u32) << 5);
const SVC20_A64: u32 = 0xD4000001 | ((SYS_FS_OPEN as u32) << 5);
const SVC21_A64: u32 = 0xD4000001 | ((SYS_FS_READ as u32) << 5);
const SVC22_A64: u32 = 0xD4000001 | ((SYS_FS_WRITE as u32) << 5);
const SVC23_A64: u32 = 0xD4000001 | ((SYS_FS_CLOSE as u32) << 5);

const EL0_FS_PATH: &[u8] = b"/eprobe";
const EL0_FS_BYTES: &[u8] = b"memfs-el0";
const EL0_FS_REJECT_PATH: &[u8] = b"/kreject";

static YIELD_COUNT: AtomicU64 = AtomicU64::new(0);
static EXIT_STATUS: AtomicU64 = AtomicU64::new(u64::MAX);
static UART_LAST: AtomicU64 = AtomicU64::new(u64::MAX);
static YIELD_OK: AtomicBool = AtomicBool::new(false);
static UART_OK: AtomicBool = AtomicBool::new(false);
static EXIT_OK: AtomicBool = AtomicBool::new(false);
static FS_CREATE: AtomicU64 = AtomicU64::new(0);
static FS_OPEN: AtomicU64 = AtomicU64::new(0);
static FS_READ: AtomicU64 = AtomicU64::new(u64::MAX);
static FS_WRITE: AtomicU64 = AtomicU64::new(u64::MAX);
static FS_CLOSE_OK: AtomicBool = AtomicBool::new(false);

/// What the lower-EL handler should do after a public ABI call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SvcAction {
    StayEl0,
    ReturnEl1,
}

#[cfg(test)]
fn svc_a64(imm: u64) -> u32 {
    0xD4000001 | ((imm as u32) << 5)
}

fn movz(rd: u32, imm16: u16, lsl: u32) -> u32 {
    let hw = lsl / 16;
    0xD2800000 | (hw << 21) | ((imm16 as u32) << 5) | rd
}

fn movk(rd: u32, imm16: u16, lsl: u32) -> u32 {
    let hw = lsl / 16;
    0xF2800000 | (hw << 21) | ((imm16 as u32) << 5) | rd
}

fn sync_icache(ptr: *const u32) {
    let x = ptr as u64;
    unsafe {
        core::arch::asm!(
            "dc cvau, {x}",
            "dsb ish",
            "ic ivau, {x}",
            "dsb ish",
            "isb",
            x = in(reg) x,
            options(nostack, preserves_flags),
        );
    }
}

fn write_word(ptr: *mut u32, insn: u32) {
    unsafe {
        core::ptr::write_volatile(ptr, insn);
    }
    sync_icache(ptr);
}

/// Encode `val` into `Xd` (MOVZ + MOVK as needed). Returns words written.
fn write_imm64(mut ptr: *mut u32, rd: u32, val: u64) -> usize {
    write_word(ptr, movz(rd, val as u16, 0));
    let mut n = 1;
    for shift in [16u32, 32, 48] {
        let part = ((val >> shift) & 0xffff) as u16;
        if part != 0 {
            ptr = unsafe { ptr.add(1) };
            write_word(ptr, movk(rd, part, shift));
            n += 1;
        }
    }
    n
}

fn user_range_ok_max(ptr: u64, len: u64, max: u64) -> bool {
    if len == 0 || len > max {
        return false;
    }
    if ptr.checked_add(len).is_none() {
        return false;
    }
    let last = ptr + len - 1;
    // Walks mask to 39 bits. A TTBR1 / non-canonical alias of a mapped
    // page would otherwise pass, and `copy_user` would load the original
    // pointer (kernel abort instead of `uart_write` → 0).
    if ptr != paging::identity_pa(ptr) || last != paging::identity_pa(last) {
        return false;
    }
    let mut page = ptr & !0xfff;
    let end = last & !0xfff;
    while page <= end {
        if !paging::user_mapped(page) || !paging::is_mapped(page) {
            return false;
        }
        if page == end {
            break;
        }
        page = match page.checked_add(4096) {
            Some(p) => p,
            None => return false,
        };
    }
    true
}

fn copy_user(ptr: u64, len: usize, dst: &mut [u8]) -> Option<usize> {
    copy_user_max(ptr, len, dst, UART_WRITE_MAX)
}

fn copy_user_max(ptr: u64, len: usize, dst: &mut [u8], max: u64) -> Option<usize> {
    if len > dst.len() || !user_range_ok_max(ptr, len as u64, max) {
        return None;
    }
    for i in 0..len {
        dst[i] = unsafe { core::ptr::read_volatile((ptr as *const u8).add(i)) };
    }
    Some(len)
}

fn copy_to_user(ptr: u64, src: &[u8]) -> Option<usize> {
    if src.is_empty() {
        return Some(0);
    }
    if !user_range_ok_max(ptr, src.len() as u64, FS_IO_MAX) {
        return None;
    }
    for (i, &b) in src.iter().enumerate() {
        unsafe {
            core::ptr::write_volatile((ptr as *mut u8).add(i), b);
        }
    }
    Some(src.len())
}

fn path_from_user(ptr: u64, len: u64) -> Option<[u8; vfs::PATH_MAX]> {
    if len == 0 || len > vfs::PATH_MAX as u64 {
        return None;
    }
    let mut buf = [0u8; vfs::PATH_MAX];
    copy_user_max(ptr, len as usize, &mut buf, vfs::PATH_MAX as u64)?;
    Some(buf)
}

fn path_str(buf: &[u8], len: u64) -> Option<&str> {
    let n = len as usize;
    if n > buf.len() {
        return None;
    }
    let s = core::str::from_utf8(&buf[..n]).ok()?;
    if vfs::valid_path(s) {
        Some(s)
    } else {
        None
    }
}

fn mov_reg(rd: u32, rn: u32) -> u32 {
    // `MOV Xd, Xn` == `ORR Xd, XZR, Xn`
    0xAA0003E0 | (rn << 16) | rd
}

/// Dispatch a public ABI `SVC`. Probe immediates 0–2 stay in `exception`.
pub fn dispatch(ctx: &mut ExceptionContext) -> Option<SvcAction> {
    let nr = ctx.esr & 0xffff;
    match nr {
        SYS_YIELD => {
            el0::note_active_while_standing();
            YIELD_COUNT.fetch_add(1, Ordering::SeqCst);
            YIELD_OK.store(true, Ordering::SeqCst);
            ctx.x[0] = 0;
            uart::write_str_raw("svc: yield\n");
            Some(SvcAction::StayEl0)
        }
        SYS_UART_WRITE => {
            el0::note_active_while_standing();
            let ptr = ctx.x[0];
            let len = ctx.x[1];
            let mut buf = [0u8; UART_WRITE_MAX as usize];
            match copy_user(ptr, len as usize, &mut buf) {
                Some(n) => {
                    uart::write_bytes_raw(&buf[..n]);
                    UART_LAST.store(n as u64, Ordering::SeqCst);
                    UART_OK.store(true, Ordering::SeqCst);
                    ctx.x[0] = n as u64;
                    let mut w = uart::raw();
                    let _ = writeln!(w, "svc: uart n={n}");
                    Some(SvcAction::StayEl0)
                }
                None => {
                    UART_LAST.store(0, Ordering::SeqCst);
                    ctx.x[0] = 0;
                    Some(SvcAction::StayEl0)
                }
            }
        }
        SYS_EXIT => {
            let status = ctx.x[0];
            EXIT_STATUS.store(status, Ordering::SeqCst);
            EXIT_OK.store(true, Ordering::SeqCst);
            uart::write_str_raw("svc: exit\n");
            Some(SvcAction::ReturnEl1)
        }
        SYS_FS_CREATE => {
            el0::note_active_while_standing();
            let ptr = ctx.x[0];
            let len = ctx.x[1];
            let fd = path_from_user(ptr, len)
                .and_then(|buf| path_str(&buf, len).and_then(|p| vfs::create(p).ok()))
                .unwrap_or(0);
            FS_CREATE.store(fd as u64, Ordering::SeqCst);
            ctx.x[0] = fd as u64;
            Some(SvcAction::StayEl0)
        }
        SYS_FS_OPEN => {
            el0::note_active_while_standing();
            let ptr = ctx.x[0];
            let len = ctx.x[1];
            let fd = path_from_user(ptr, len)
                .and_then(|buf| path_str(&buf, len).and_then(|p| vfs::open(p).ok()))
                .unwrap_or(0);
            FS_OPEN.store(fd as u64, Ordering::SeqCst);
            ctx.x[0] = fd as u64;
            Some(SvcAction::StayEl0)
        }
        SYS_FS_READ => {
            el0::note_active_while_standing();
            let fd = ctx.x[0] as u32;
            let ptr = ctx.x[1];
            let len = ctx.x[2];
            if len == 0 || len > FS_IO_MAX || !user_range_ok_max(ptr, len, FS_IO_MAX) {
                FS_READ.store(FS_ERR, Ordering::SeqCst);
                ctx.x[0] = FS_ERR;
                return Some(SvcAction::StayEl0);
            }
            let mut tmp = [0u8; FS_IO_MAX as usize];
            match vfs::read(fd, &mut tmp[..len as usize]) {
                Ok(n) => match copy_to_user(ptr, &tmp[..n]) {
                    Some(_) => {
                        if n > 0 {
                            vfs::note_el0_read(&tmp[..n]);
                        }
                        FS_READ.store(n as u64, Ordering::SeqCst);
                        ctx.x[0] = n as u64;
                    }
                    None => {
                        FS_READ.store(FS_ERR, Ordering::SeqCst);
                        ctx.x[0] = FS_ERR;
                    }
                },
                Err(_) => {
                    FS_READ.store(FS_ERR, Ordering::SeqCst);
                    ctx.x[0] = FS_ERR;
                }
            }
            Some(SvcAction::StayEl0)
        }
        SYS_FS_WRITE => {
            el0::note_active_while_standing();
            let fd = ctx.x[0] as u32;
            let ptr = ctx.x[1];
            let len = ctx.x[2];
            let mut tmp = [0u8; FS_IO_MAX as usize];
            let n = match copy_user_max(ptr, len as usize, &mut tmp, FS_IO_MAX)
                .and_then(|n| vfs::write(fd, &tmp[..n]).ok())
            {
                Some(n) => n as u64,
                None => 0,
            };
            FS_WRITE.store(n, Ordering::SeqCst);
            ctx.x[0] = n;
            Some(SvcAction::StayEl0)
        }
        SYS_FS_CLOSE => {
            el0::note_active_while_standing();
            match vfs::close(ctx.x[0] as u32) {
                Ok(()) => {
                    FS_CLOSE_OK.store(true, Ordering::SeqCst);
                    ctx.x[0] = 0;
                }
                Err(_) => {
                    ctx.x[0] = FS_ERR;
                }
            }
            Some(SvcAction::StayEl0)
        }
        _ => None,
    }
}

pub(crate) fn reset_probe_flags() {
    YIELD_COUNT.store(0, Ordering::SeqCst);
    EXIT_STATUS.store(u64::MAX, Ordering::SeqCst);
    UART_LAST.store(u64::MAX, Ordering::SeqCst);
    YIELD_OK.store(false, Ordering::SeqCst);
    UART_OK.store(false, Ordering::SeqCst);
    EXIT_OK.store(false, Ordering::SeqCst);
    reset_fs_flags();
}

pub(crate) fn reset_fs_flags() {
    FS_CREATE.store(0, Ordering::SeqCst);
    FS_OPEN.store(0, Ordering::SeqCst);
    FS_READ.store(u64::MAX, Ordering::SeqCst);
    FS_WRITE.store(u64::MAX, Ordering::SeqCst);
    FS_CLOSE_OK.store(false, Ordering::SeqCst);
}

fn write_bytes(base: *mut u8, off: usize, bytes: &[u8]) {
    for (i, &b) in bytes.iter().enumerate() {
        unsafe {
            core::ptr::write_volatile(base.add(off + i), b);
        }
    }
}

/// EL0: create `/eprobe`, write `memfs-el0`, close, open, read, close, exit.
fn write_fs_success_payload(ptr: *mut u32, va: u64) {
    let path_off = 256usize;
    let data_off = 272usize;
    let buf_off = 288usize;
    write_bytes(ptr as *mut u8, path_off, EL0_FS_PATH);
    write_bytes(ptr as *mut u8, data_off, EL0_FS_BYTES);
    let mut p = ptr;
    let n = write_imm64(p, 0, va + path_off as u64);
    p = unsafe { p.add(n) };
    write_word(p, movz(1, EL0_FS_PATH.len() as u16, 0));
    p = unsafe { p.add(1) };
    write_word(p, SVC19_A64);
    p = unsafe { p.add(1) };
    write_word(p, mov_reg(3, 0));
    p = unsafe { p.add(1) };
    write_word(p, mov_reg(0, 3));
    p = unsafe { p.add(1) };
    let n = write_imm64(p, 1, va + data_off as u64);
    p = unsafe { p.add(n) };
    write_word(p, movz(2, EL0_FS_BYTES.len() as u16, 0));
    p = unsafe { p.add(1) };
    write_word(p, SVC22_A64);
    p = unsafe { p.add(1) };
    write_word(p, mov_reg(0, 3));
    p = unsafe { p.add(1) };
    write_word(p, SVC23_A64);
    p = unsafe { p.add(1) };
    let n = write_imm64(p, 0, va + path_off as u64);
    p = unsafe { p.add(n) };
    write_word(p, movz(1, EL0_FS_PATH.len() as u16, 0));
    p = unsafe { p.add(1) };
    write_word(p, SVC20_A64);
    p = unsafe { p.add(1) };
    write_word(p, mov_reg(3, 0));
    p = unsafe { p.add(1) };
    write_word(p, mov_reg(0, 3));
    p = unsafe { p.add(1) };
    let n = write_imm64(p, 1, va + buf_off as u64);
    p = unsafe { p.add(n) };
    write_word(p, movz(2, 16, 0));
    p = unsafe { p.add(1) };
    write_word(p, SVC21_A64);
    p = unsafe { p.add(1) };
    write_word(p, mov_reg(0, 3));
    p = unsafe { p.add(1) };
    write_word(p, SVC23_A64);
    p = unsafe { p.add(1) };
    write_word(p, movz(0, 0, 0));
    p = unsafe { p.add(1) };
    write_word(p, SVC16_A64);
}

/// EL0: create `/kreject`, `SYS_FS_WRITE` from kernel `.data`, then exit.
fn write_fs_reject_payload(ptr: *mut u32, va: u64, bait: u64) {
    let path_off = 256usize;
    write_bytes(ptr as *mut u8, path_off, EL0_FS_REJECT_PATH);
    let mut p = ptr;
    let n = write_imm64(p, 0, va + path_off as u64);
    p = unsafe { p.add(n) };
    write_word(p, movz(1, EL0_FS_REJECT_PATH.len() as u16, 0));
    p = unsafe { p.add(1) };
    write_word(p, SVC19_A64);
    p = unsafe { p.add(1) };
    let n = write_imm64(p, 1, bait);
    p = unsafe { p.add(n) };
    write_word(p, movz(2, 4, 0));
    p = unsafe { p.add(1) };
    write_word(p, SVC22_A64);
    p = unsafe { p.add(1) };
    write_word(p, movz(0, 0, 0));
    p = unsafe { p.add(1) };
    write_word(p, SVC16_A64);
}

pub(crate) fn run_fs_el0_payload() -> bool {
    if el0::is_active() || !paging::user_map_ready() {
        return false;
    }
    reset_probe_flags();
    with_el0_page(|ptr, va| {
        write_fs_success_payload(ptr, va);
        let user_sp = va + 4096;
        el0::install_standing(va, user_sp, paging::user_ttbr0());
        unsafe {
            exception::eret_to_el0(black_box(va), 0, user_sp);
        }
        if el0::is_active() {
            el0::clear_active();
            return false;
        }
        EXIT_OK.load(Ordering::SeqCst)
    })
}

#[allow(dead_code)] // `#[test_case]` only.
pub(crate) fn fs_el0_reject_kernel_data() -> bool {
    let bait = paging::identity_pa(core::ptr::addr_of!(crate::el0::KERNEL_DATA_BAIT) as u64);
    if paging::user_mapped(bait) {
        return false;
    }
    vfs::reset();
    reset_probe_flags();
    if !run_payload(|ptr, va| write_fs_reject_payload(ptr, va, bait)) {
        return false;
    }
    last_fs_create() >= 1 && last_fs_write() == 0
}

fn with_el0_page<F: FnOnce(*mut u32, u64) -> bool>(f: F) -> bool {
    let Some(pa) = frame::alloc() else {
        return false;
    };
    let va = paging::EL0_PAGE;
    if !paging::map_el0_exec(va, pa) {
        frame::free(pa);
        return false;
    }
    let ok = f(va as *mut u32, va);
    let _ = paging::unmap_page(va);
    frame::free(pa);
    ok
}

/// `SVC #18` / load user ptr+len / `SVC #17` / `SVC #16` with status 0.
fn write_success_payload(ptr: *mut u32, va: u64) {
    let msg_off = 64u64;
    let msg_va = va + msg_off;
    let mut p = ptr;
    write_word(p, SVC18_A64);
    p = unsafe { p.add(1) };
    let n = write_imm64(p, 0, msg_va);
    p = unsafe { p.add(n) };
    write_word(p, movz(1, USER_UART_MSG.len() as u16, 0));
    p = unsafe { p.add(1) };
    write_word(p, SVC17_A64);
    p = unsafe { p.add(1) };
    write_word(p, movz(0, 0, 0));
    p = unsafe { p.add(1) };
    write_word(p, SVC16_A64);
    let dst = unsafe { (ptr as *mut u8).add(msg_off as usize) };
    for (i, &b) in USER_UART_MSG.iter().enumerate() {
        unsafe {
            core::ptr::write_volatile(dst.add(i), b);
        }
    }
}

/// `SYS_UART_WRITE` from kernel `.data`, then `SYS_EXIT`.
#[allow(dead_code)] // hello kernel uses the success trip only.
fn write_reject_payload(ptr: *mut u32, bait: u64) {
    let mut p = ptr;
    let n = write_imm64(p, 0, bait);
    p = unsafe { p.add(n) };
    write_word(p, movz(1, 4, 0));
    p = unsafe { p.add(1) };
    write_word(p, SVC17_A64);
    p = unsafe { p.add(1) };
    write_word(p, movz(0, 0, 0));
    p = unsafe { p.add(1) };
    write_word(p, SVC16_A64);
}

fn run_payload<F: FnOnce(*mut u32, u64)>(write: F) -> bool {
    if el0::is_active() || !paging::user_map_ready() {
        return false;
    }
    reset_probe_flags();
    with_el0_page(|ptr, va| {
        write(ptr, va);
        let user_sp = va + 4096;
        el0::install_standing(va, user_sp, paging::user_ttbr0());
        unsafe {
            exception::eret_to_el0(black_box(va), 0, user_sp);
        }
        if el0::is_active() {
            el0::clear_active();
            return false;
        }
        EXIT_OK.load(Ordering::SeqCst)
    })
}

fn abi_success_trip() -> bool {
    if !run_payload(|ptr, va| write_success_payload(ptr, va)) {
        return false;
    }
    YIELD_OK.load(Ordering::SeqCst)
        && UART_OK.load(Ordering::SeqCst)
        && UART_LAST.load(Ordering::SeqCst) == USER_UART_MSG.len() as u64
        && EXIT_STATUS.load(Ordering::SeqCst) == 0
        && YIELD_COUNT.load(Ordering::SeqCst) >= 1
}

#[allow(dead_code)] // `#[test_case]` only.
fn abi_reject_kernel_data() -> bool {
    let bait = paging::identity_pa(core::ptr::addr_of!(crate::el0::KERNEL_DATA_BAIT) as u64);
    if paging::user_mapped(bait) {
        return false;
    }
    if !run_payload(|ptr, _va| write_reject_payload(ptr, bait)) {
        return false;
    }
    UART_LAST.load(Ordering::SeqCst) == 0 && !UART_OK.load(Ordering::SeqCst)
}

/// `SYS_UART_WRITE` via the TTBR1 alias of a user-mapped page, then `SYS_EXIT`.
#[allow(dead_code)] // `#[test_case]` only.
fn abi_reject_high_alias() -> bool {
    if !run_payload(|ptr, va| {
        let msg_off = 64u64;
        let dst = unsafe { (ptr as *mut u8).add(msg_off as usize) };
        for i in 0..4 {
            unsafe {
                core::ptr::write_volatile(dst.add(i), b'X');
            }
        }
        write_reject_payload(ptr, paging::to_high_va(va + msg_off));
    }) {
        return false;
    }
    UART_LAST.load(Ordering::SeqCst) == 0 && !UART_OK.load(Ordering::SeqCst)
}

/// Serial proof: EL0 issues yield, uart_write, exit. Not app hosting.
#[allow(dead_code)] // hello kernel only; cargo test uses the cases below.
pub fn observe_probe() -> bool {
    if !paging::mmu_enabled() || !paging::user_map_ready() {
        return false;
    }
    if !abi_success_trip() {
        return false;
    }
    let mut w = uart::raw();
    let _ = writeln!(w, "svc: ok");
    true
}

#[allow(dead_code)]
pub fn yield_count() -> u64 {
    YIELD_COUNT.load(Ordering::SeqCst)
}

#[allow(dead_code)]
pub fn last_uart_write() -> u64 {
    UART_LAST.load(Ordering::SeqCst)
}

#[allow(dead_code)]
pub fn last_exit_status() -> u64 {
    EXIT_STATUS.load(Ordering::SeqCst)
}

#[allow(dead_code)]
pub(crate) fn yield_seen() -> bool {
    YIELD_OK.load(Ordering::SeqCst)
}

#[allow(dead_code)]
pub(crate) fn uart_seen() -> bool {
    UART_OK.load(Ordering::SeqCst)
}

#[allow(dead_code)]
pub(crate) fn exit_seen() -> bool {
    EXIT_OK.load(Ordering::SeqCst)
}

pub(crate) fn last_fs_create() -> u64 {
    FS_CREATE.load(Ordering::SeqCst)
}

pub(crate) fn last_fs_open() -> u64 {
    FS_OPEN.load(Ordering::SeqCst)
}

pub(crate) fn last_fs_read() -> u64 {
    FS_READ.load(Ordering::SeqCst)
}

pub(crate) fn last_fs_write() -> u64 {
    FS_WRITE.load(Ordering::SeqCst)
}

pub(crate) fn fs_close_seen() -> bool {
    FS_CLOSE_OK.load(Ordering::SeqCst)
}

#[cfg(test)]
#[test_case]
fn syscall_numbers_are_documented() {
    assert_eq!(svc_a64(SYS_EXIT), SVC16_A64);
    assert_eq!(svc_a64(SYS_UART_WRITE), SVC17_A64);
    assert_eq!(svc_a64(SYS_YIELD), SVC18_A64);
    assert_eq!(svc_a64(SYS_FS_CREATE), SVC19_A64);
    assert_eq!(svc_a64(SYS_FS_OPEN), SVC20_A64);
    assert_eq!(svc_a64(SYS_FS_READ), SVC21_A64);
    assert_eq!(svc_a64(SYS_FS_WRITE), SVC22_A64);
    assert_eq!(svc_a64(SYS_FS_CLOSE), SVC23_A64);
    assert_ne!(SYS_EXIT, SVC_PROBE_RETURN);
    assert_ne!(SYS_EXIT, SVC_PROBE_STANDING);
    assert_ne!(SYS_EXIT, SVC_PROBE_RESTORE);
}

#[cfg(test)]
#[test_case]
fn el0_svc_abi_yield_uart_exit() {
    assert!(paging::mmu_enabled());
    assert!(
        abi_success_trip(),
        "EL0 must SVC yield, uart_write, then exit"
    );
    assert!(!el0::is_active());
    assert_eq!(last_uart_write(), USER_UART_MSG.len() as u64);
    assert_eq!(last_exit_status(), 0);
}

#[cfg(test)]
#[test_case]
fn el0_uart_write_rejects_kernel_data() {
    assert!(paging::mmu_enabled());
    assert!(
        abi_reject_kernel_data(),
        "SYS_UART_WRITE must reject a kernel .data pointer"
    );
    assert!(!el0::is_active());
}

#[cfg(test)]
#[test_case]
fn el0_uart_write_rejects_high_alias() {
    assert!(paging::mmu_enabled());
    assert!(
        abi_reject_high_alias(),
        "SYS_UART_WRITE must reject a TTBR1 alias of a user-mapped page"
    );
    assert!(!el0::is_active());
}
