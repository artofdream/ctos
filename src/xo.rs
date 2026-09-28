//! ADR-097: EL0 app text is execute-only; `.rodata` lives on the app-hdr page.
//!
//! Layout (every app, the A3 loader and the A2 memcpy path):
//! - `app-hdr` slot (`MAP_WINDOW`, 0x80000000): ELF headers + `.rodata`, EL0
//!   read-only, NX (AP[2:1]=11, UXN set);
//! - `app-text` slot (`EL0_PAGE`, 0x80002000): `.text` only, EL0 execute-only
//!   (AP[2:1]=10, UXN clear, PXN set). EL0 cannot load or store it.
//! No new slot: the ADR-092 allowlist stays at five.
//!
//! Probes (fail-closed; `xo: ok` only if all hold):
//! 1. **live** (`on_live`, from the ADR-092 live walk while `hello-libctos`
//!    stands at EL0): the real app's text leaf is EL0 execute-only and still
//!    EL0-reachable (UXN clear), its app-hdr leaf is EL0 read-only NX and
//!    holds `libctos: hi\n`, the text frame does not, and the ADR-094 read
//!    check refuses a text pointer but allows the rodata pointer;
//! 2. **loader image**: a small ELF with the same layout, mapped by the real
//!    loader (`loader::load_only`). EL0 executes its text, loads a sentinel from its
//!    rodata, takes a permission fault loading its own text (ESR/FAR shown),
//!    prints a rodata string through `uart_write`, and has `uart_write` of a
//!    text pointer refused by the ADR-094 check (one refusal, nothing printed);
//! 3. **walk**: an execute-only leaf planted off the allowlist is caught
//!    (`why=va`), so the UXN half of the EL0-reachable rule is live;
//! 4. **EL1 / PAN facts**: with PAN set, an EL1 load of an EL0-readable page
//!    faults (control), an EL1 load of the execute-only text does **not**
//!    fault (no FEAT_EPAN: ID_AA64MMFR1_EL1.PAN = 2 on `-cpu cortex-a76`), and
//!    an EL1 store to the text VA faults (AP[2]=1, not PAN).
//!
//! 5. **PAN held in handlers** (live, in the `SYS_YIELD` handler): PSTATE.PAN
//!    is set and SCTLR_EL1.SPAN is clear (ADR-097 PAN fix), and the control
//!    load in (4) runs after EL0 trips.
//!
//! The `xo-leak-probe` build maps app text EL0-readable again; the probe must
//! print `xo: leak …` and withhold `xo: ok`. The `pan-keep-leak-probe` build
//! undoes the PAN fix; the probe must print `xo: bad pan …` / `xo: bad el1-pan
//! control …` and withhold `xo: ok`.
//!
//! Limits: one core, QEMU TCG. PAN does not cover the execute-only page, so
//! "EL1 faults on EL0 memory" does not hold for it (see ADR-097). EL0 can
//! still observe its own code indirectly (timing, fault addresses); this is
//! a page-permission property, not a side-channel claim.

use alloc::vec::Vec;
use core::fmt::Write;
use core::hint::black_box;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::exception;
use crate::frame;
use crate::loader;
use crate::paging;
use crate::syscall;
use crate::uart;

const TEXT_VA: u64 = paging::EL0_PAGE;
const HDR_VA: u64 = paging::MAP_WINDOW;
const RO_OFF: u64 = 0x800;
const RO_VA: u64 = HDR_VA + RO_OFF;
const RO_SENT: u64 = 0x584f_5244_4154_4131;
const RO_MSG_OFF: u64 = 0x810;
const RO_MSG_VA: u64 = HDR_VA + RO_MSG_OFF;
const RO_MSG: &[u8] = b"xo: el0-uart rodata-str ok\n";
const TEXT_FILE_OFF: usize = 0x1000;
const TEXT_LEN: usize = 0x100;
/// Code offsets inside the text page.
const OFF_LOAD: u64 = 0x00;
const OFF_SYS_RO: u64 = 0x40;
const OFF_SYS_TEXT: u64 = 0x80;
/// Unused byte in the text page for the EL1 store probe.
const EL1_STORE_OFF: u64 = 0xf00;

const LDR_X1_X0: u32 = 0xF940_0001;
const BRK0: u32 = 0xD420_0000;
const SVC_EXIT: u32 = 0xD400_0201; // SVC #16
const SVC_UART: u32 = 0xD400_0221; // SVC #17
const EC_DABORT_LOWER: u64 = 0x24;
const EC_BRK: u64 = 0x3C;

static LIVE_DONE: AtomicBool = AtomicBool::new(false);
static LIVE_OK: AtomicBool = AtomicBool::new(false);

fn movz(rd: u32, imm: u16, hw: u32) -> u32 {
    0xD280_0000 | (hw << 21) | ((imm as u32) << 5) | rd
}

fn movk(rd: u32, imm: u16, hw: u32) -> u32 {
    0xF280_0000 | (hw << 21) | ((imm as u32) << 5) | rd
}

fn put32(b: &mut [u8], off: usize, v: u32) {
    b[off..off + 4].copy_from_slice(&v.to_le_bytes());
}

fn put16(b: &mut [u8], off: usize, v: u16) {
    b[off..off + 2].copy_from_slice(&v.to_le_bytes());
}

fn put64(b: &mut [u8], off: usize, v: u64) {
    b[off..off + 8].copy_from_slice(&v.to_le_bytes());
}

/// `x0 = va (32-bit); x1 = len; SVC #17; x0 = 0; SVC #16`.
fn code_uart(va: u64, len: u16) -> [u32; 6] {
    [
        movz(0, (va & 0xffff) as u16, 0),
        movk(0, ((va >> 16) & 0xffff) as u16, 1),
        movz(1, len, 0),
        SVC_UART,
        movz(0, 0, 0),
        SVC_EXIT,
    ]
}

/// Code at `OFF_LOAD`: `MOVZ X1, #0; LDR X1, [X0]; BRK #0` (load at +4).
fn code_load() -> [u32; 3] {
    [movz(1, 0, 0), LDR_X1_X0, BRK0]
}

/// An app-shaped ELF: R `PT_LOAD` of headers + rodata at 0x80000000 and an
/// R+X `PT_LOAD` of text at 0x80002000 (what the user linker scripts emit).
fn build_elf() -> Vec<u8> {
    let mut e = alloc::vec![0u8; TEXT_FILE_OFF + TEXT_LEN];
    e[0..4].copy_from_slice(b"\x7fELF");
    e[4] = 2; // ELFCLASS64
    e[5] = 1; // little-endian
    e[6] = 1; // EV_CURRENT
    put16(&mut e, 16, 2); // ET_EXEC
    put16(&mut e, 18, 183); // EM_AARCH64
    put32(&mut e, 20, 1);
    put64(&mut e, 24, TEXT_VA);
    put64(&mut e, 32, 64); // e_phoff
    put16(&mut e, 52, 64); // e_ehsize
    put16(&mut e, 54, 56); // e_phentsize
    put16(&mut e, 56, 2); // e_phnum
    let ro_len = RO_MSG_OFF + RO_MSG.len() as u64;
    for (i, (flags, off, va, len)) in [(4u32, 0u64, HDR_VA, ro_len), (5, TEXT_FILE_OFF as u64, TEXT_VA, TEXT_LEN as u64)]
        .iter()
        .enumerate()
    {
        let p = 64 + i * 56;
        put32(&mut e, p, 1); // PT_LOAD
        put32(&mut e, p + 4, *flags);
        put64(&mut e, p + 8, *off);
        put64(&mut e, p + 16, *va);
        put64(&mut e, p + 24, *va);
        put64(&mut e, p + 32, *len);
        put64(&mut e, p + 40, *len);
        put64(&mut e, p + 48, 0x1000);
    }
    put64(&mut e, RO_OFF as usize, RO_SENT);
    e[RO_MSG_OFF as usize..RO_MSG_OFF as usize + RO_MSG.len()].copy_from_slice(RO_MSG);
    let t = TEXT_FILE_OFF;
    for (i, w) in code_load().iter().enumerate() {
        put32(&mut e, t + OFF_LOAD as usize + 4 * i, *w);
    }
    for (i, w) in code_uart(RO_MSG_VA, RO_MSG.len() as u16).iter().enumerate() {
        put32(&mut e, t + OFF_SYS_RO as usize + 4 * i, *w);
    }
    for (i, w) in code_uart(TEXT_VA, 8).iter().enumerate() {
        put32(&mut e, t + OFF_SYS_TEXT as usize + 4 * i, *w);
    }
    e
}

fn el0_str(v: paging::LeafView) -> &'static str {
    match (v.el0_read, v.el0_write, v.el0_exec) {
        (false, false, true) => "x",
        (true, false, false) => "r",
        (true, false, true) => "rx",
        (true, true, false) => "rw",
        (true, true, true) => "rwx",
        (false, false, false) => "none",
        _ => "odd",
    }
}

fn ec(esr: u64) -> u64 {
    (esr >> 26) & 0x3f
}

/// Permission-fault level 0..=3 (DFSC 0b0011xx), or `None`.
fn perm_level(esr: u64) -> Option<u64> {
    let dfsc = esr & 0x3f;
    if (0x0c..=0x0f).contains(&dfsc) {
        Some(dfsc - 0x0c)
    } else {
        None
    }
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

fn frame_bytes(leaf: u64) -> &'static [u8] {
    let pa = leaf & 0x0000_ffff_ffff_f000;
    unsafe { core::slice::from_raw_parts(paging::frame_cpu_va(pa) as *const u8, 4096) }
}

/// ADR-097 live check, called from the ADR-092 live walk (`SYS_YIELD` of the
/// real `hello-libctos`, all its pages mapped by the loader).
pub fn on_live() {
    LIVE_DONE.store(true, Ordering::SeqCst);
    // ADR-097 PAN fix: this runs in the SYS_YIELD handler while the app
    // stands at EL0. With SCTLR_EL1.SPAN=0 the exception set PSTATE.PAN.
    // A faulting EL1 load cannot be used here (a current-EL abort inside a
    // handler is fatal by design), so read PSTATE.PAN with `MRS PAN`; the
    // image probe below calibrates that read against a real EL1 fault.
    let span = crate::pan::sctlr_span();
    let pstate = crate::pan::pstate_pan();
    let pan_live_ok = pstate == 1 && span == 0;
    {
        let mut w = uart::raw();
        if !pan_live_ok {
            let _ = writeln!(w, "xo: bad pan live-syscall pstate-pan={} sctlr-span={}", pstate, span);
        } else {
            let _ = writeln!(w, "xo: pan live-syscall pstate-pan=1 sctlr-span=0");
        }
    }
    let text = paging::l3_entry(TEXT_VA).unwrap_or(0);
    let hdr = paging::l3_entry(HDR_VA).unwrap_or(0);
    let mut w = uart::raw();
    if text & 1 == 0 || hdr & 1 == 0 {
        let _ = writeln!(w, "xo: bad live text={:#x} hdr={:#x}", text, hdr);
        LIVE_OK.store(false, Ordering::SeqCst);
        return;
    }
    let tv = paging::leaf_view(text);
    let hv = paging::leaf_view(hdr);
    let needle = b"libctos: hi\n";
    let ro_off = find(frame_bytes(hdr), needle);
    let in_text = find(frame_bytes(text), needle).is_some();
    let sys_text = syscall::el0_read_allowed(TEXT_VA, 8);
    let sys_ro = ro_off.map(|o| syscall::el0_read_allowed(HDR_VA + o as u64, needle.len() as u64)).unwrap_or(false);
    if tv.el0_read || sys_text {
        let _ = writeln!(w, "xo: leak live text ap={} el0={} sys-read={}", tv.ap, el0_str(tv), if sys_text { "allow" } else { "deny" });
        LIVE_OK.store(false, Ordering::SeqCst);
        return;
    }
    let ok = tv.el0_exec
        && !tv.el0_write
        && tv.reachable
        && tv.pxn
        && tv.ap == 2
        && hv.el0_read
        && !hv.el0_write
        && !hv.el0_exec
        && ro_off.is_some()
        && !in_text
        && sys_ro
        && pan_live_ok;
    if !ok {
        let _ = writeln!(
            w,
            "xo: bad live text ap={} el0={} hdr ap={} el0={} rodata={:?} in-text={} sys-ro={}",
            tv.ap,
            el0_str(tv),
            hv.ap,
            el0_str(hv),
            ro_off,
            in_text,
            sys_ro
        );
        LIVE_OK.store(false, Ordering::SeqCst);
        return;
    }
    let _ = writeln!(
        w,
        "xo: live app=hello text va={:#x} ap=2 uxn=0 pxn=1 el0=x reach=1 hdr va={:#x} ap={} uxn={} el0=r rodata={:#x} in-text=0 sys-read text=deny rodata=allow",
        TEXT_VA,
        HDR_VA,
        hv.ap,
        hv.uxn as u8,
        HDR_VA + ro_off.unwrap_or(0) as u64
    );
    LIVE_OK.store(true, Ordering::SeqCst);
}

type Cap = (bool, u64, u64, u64, u64);

fn trip_load(stack_top: u64, arg: u64) -> Cap {
    exception::arm_el0_capture();
    unsafe {
        exception::eret_to_el0_masked(black_box(TEXT_VA + OFF_LOAD), arg, stack_top);
    }
    exception::el0_capture_result()
}

/// EL1 load of `va` with the ADR-080 catcher armed (quiet). `(faulted,
/// spsr_pan_at_fault, value)`; `value` is the poison when it faulted.
fn el1_load(va: u64) -> (bool, bool, u64) {
    exception::arm_el1_perm_fault_quiet();
    let mut v: u64 = 0xDEAD_BEEF_DEAD_BEEF;
    unsafe {
        core::arch::asm!("ldr {v}, [{p}]", p = in(reg) black_box(va), v = inout(reg) v, options(nostack));
    }
    let caught = exception::pan_el1_fault_caught();
    (caught, exception::pan_el1_fault_spsr_pan(), v)
}

fn el1_store(va: u64, val: u64) -> bool {
    exception::arm_el1_perm_fault_quiet();
    unsafe {
        core::arch::asm!("str {v}, [{p}]", p = in(reg) black_box(va), v = in(reg) val, options(nostack));
    }
    exception::pan_el1_fault_caught()
}

/// Steps 2-4 against a loaded app-shaped image. Keeps going after a leak so
/// the negative build reports every case.
fn image_probe(l: &loader::Loaded) -> bool {
    let mut w = uart::raw();
    let stack_top = l.stack_va + 4096;
    let (Some(text_pa), Some(hdr_pa)) = (l.pa_of(TEXT_VA), l.pa_of(HDR_VA)) else {
        uart::write_str_raw("xo: bad image pages\n");
        return false;
    };
    let text = paging::l3_entry(TEXT_VA).unwrap_or(0);
    let hdr = paging::l3_entry(HDR_VA).unwrap_or(0);
    let (tv, hv) = (paging::leaf_view(text), paging::leaf_view(hdr));
    let mut ok = true;
    if tv.el0_read {
        let _ = writeln!(w, "xo: leak layout text ap={} el0={}", tv.ap, el0_str(tv));
        ok = false;
    } else if !(tv.el0_exec && tv.reachable && tv.ap == 2 && hv.el0_read && !hv.el0_exec && !hv.el0_write) {
        let _ = writeln!(w, "xo: bad layout text ap={} el0={} hdr ap={} el0={}", tv.ap, el0_str(tv), hv.ap, el0_str(hv));
        return false;
    } else {
        let _ = writeln!(
            w,
            "xo: layout loader app-hdr va={:#x} ap={} uxn={} el0=r app-text va={:#x} ap=2 uxn=0 pxn={} el0=x reach=uxn",
            HDR_VA,
            hv.ap,
            hv.uxn as u8,
            TEXT_VA,
            tv.pxn as u8
        );
    }

    // EL0 executes its text and loads its own rodata sentinel.
    let (taken, esr, _far, elr, x1) = trip_load(stack_top, RO_VA);
    if !taken || ec(esr) != EC_BRK || x1 != RO_SENT || elr != TEXT_VA + OFF_LOAD + 8 {
        let _ = writeln!(w, "xo: bad el0-rodata taken={} esr={:#x} elr={:#x} x1={:#x}", taken, esr, elr, x1);
        return false;
    }
    let _ = writeln!(w, "xo: el0-exec ok entry={:#x} el0-rodata va={:#x} got={:#x}", TEXT_VA + OFF_LOAD, RO_VA, x1);

    // EL0 loads its own text: must be a permission fault, nothing loaded.
    let (taken, esr, far, elr, x1) = trip_load(stack_top, TEXT_VA);
    if taken && ec(esr) == EC_BRK {
        let _ = writeln!(w, "xo: leak el0-read-text va={:#x} got={:#x}", TEXT_VA, x1);
        ok = false;
    } else if !taken
        || ec(esr) != EC_DABORT_LOWER
        || (esr >> 6) & 1 != 0
        || perm_level(esr).is_none()
        || far != TEXT_VA
        || elr != TEXT_VA + OFF_LOAD + 4
        || x1 != 0
    {
        let _ = writeln!(w, "xo: bad el0-read-text taken={} esr={:#x} far={:#x} elr={:#x} x1={:#x}", taken, esr, far, elr, x1);
        return false;
    } else {
        let _ = writeln!(
            w,
            "xo: el0-read-text fault va={:#x} esr={:#x} far={:#x} ec=0x24 wnr=0 dfsc=perm-l{} got=0x0",
            TEXT_VA,
            esr,
            far,
            perm_level(esr).unwrap_or(9)
        );
    }

    // uart_write of a rodata string: printed by the app itself, no refusal.
    let (exited, n, printed, checks) = syscall::run_loaded_trip(TEXT_VA + OFF_SYS_RO, stack_top);
    if !exited || n != RO_MSG.len() as u64 || !printed || checks != 0 {
        let _ = writeln!(w, "xo: bad sys-read rodata exited={} n={} checks={}", exited, n, checks);
        return false;
    }
    let _ = writeln!(w, "xo: sys-read rodata ok va={:#x} n={}", RO_MSG_VA, n);

    // uart_write of a text pointer: refused by the ADR-094 check.
    let (exited, n, printed, checks) = syscall::run_loaded_trip(TEXT_VA + OFF_SYS_TEXT, stack_top);
    if n != 0 || printed {
        let _ = writeln!(w, "\nxo: leak sys-read text n={} checks={}", n, checks);
        ok = false;
    } else if !exited || checks != 1 {
        let _ = writeln!(w, "xo: bad sys-read text exited={} checks={}", exited, checks);
        return false;
    } else {
        let _ = writeln!(w, "xo: sys-read text denied sys=uart_write nr=17 va={:#x} n=0 checks=1", TEXT_VA);
    }

    // EL1 facts with PAN set.
    let pan_on = crate::pan::pan_is_enabled();
    let mmfr1_pan = crate::pan::pan_id();
    let mrs_pan = crate::pan::pstate_pan();
    let (c_fault, c_spsr, _) = el1_load(RO_VA);
    if !pan_on || !c_fault || !c_spsr || mrs_pan != 1 {
        let _ = writeln!(w, "xo: bad el1-pan control pan={} mrs-pan={} fault={} spsr-pan={}", pan_on, mrs_pan, c_fault, c_spsr);
        return false;
    }
    let _ = writeln!(w, "xo: el1-pan control app-hdr va={:#x} mrs-pan=1 fault=yes spsr-pan=1 after-el0-trips", RO_VA);
    let word = unsafe { core::ptr::read_volatile(paging::frame_cpu_va(text_pa) as *const u64) };
    let (t_fault, _t_spsr, t_val) = el1_load(TEXT_VA);
    let epan = if mmfr1_pan >= 3 { "yes" } else { "no" };
    let _ = writeln!(
        w,
        "xo: el1-pan text va={:#x} fault={} match={} mmfr1-pan={} epan={}",
        TEXT_VA,
        if t_fault { "yes" } else { "no" },
        !t_fault && t_val == word,
        mmfr1_pan,
        epan
    );
    let before = unsafe { core::ptr::read_volatile((paging::frame_cpu_va(text_pa) + EL1_STORE_OFF) as *const u64) };
    let s_fault = el1_store(TEXT_VA + EL1_STORE_OFF, 0x5752_4954_4531_5858);
    let after = unsafe { core::ptr::read_volatile((paging::frame_cpu_va(text_pa) + EL1_STORE_OFF) as *const u64) };
    if !s_fault || after != before {
        let _ = writeln!(w, "xo: bad el1-write text fault={} intact={}", s_fault, after == before);
        return false;
    }
    let _ = writeln!(w, "xo: el1-write text va={:#x} fault=yes intact=true", TEXT_VA + EL1_STORE_OFF);
    let _ = hdr_pa;
    ok
}

/// Step 3: an execute-only leaf off the allowlist must be caught by the walk.
fn walk_probe() -> bool {
    let Some(pa) = frame::alloc() else {
        return false;
    };
    let r = paging::el0_reach_xo_plant_probe(pa);
    frame::free(pa);
    let mut w = uart::raw();
    let Some((va, r, desc)) = r else {
        uart::write_str_raw("xo: bad reach plant slot-busy\n");
        return false;
    };
    let v = paging::leaf_view(desc);
    let want = [true, true, false, false];
    if r.query_hit != want || r.query_why != "va" || v.el0_read || !v.el0_exec {
        let _ = writeln!(w, "xo: bad reach xo-plant va={:#x} why={}", va, r.query_why);
        return false;
    }
    let _ = writeln!(w, "xo: reach xo-plant caught va={:#x} roots=k,u why=va ap-el0=0 uxn=0", va);
    true
}

fn run_probe(require_live: bool) -> bool {
    if !paging::mmu_enabled() || !paging::user_map_ready() || crate::el0::is_active() {
        return false;
    }
    let mut ok = true;
    if require_live {
        if !LIVE_DONE.load(Ordering::SeqCst) {
            uart::write_str_raw("xo: live missed\n");
            return false;
        }
        ok &= LIVE_OK.load(Ordering::SeqCst);
    }
    if paging::l3_entry(TEXT_VA).unwrap_or(1) != 0 || paging::l3_entry(HDR_VA).unwrap_or(1) != 0 {
        uart::write_str_raw("xo: bad slots busy\n");
        return false;
    }
    let elf = build_elf();
    let Some(l) = loader::load_only(&elf) else {
        uart::write_str_raw("xo: bad load\n");
        return false;
    };
    if l.entry != TEXT_VA {
        loader::unload(l);
        return false;
    }
    let img = image_probe(&l);
    loader::unload(l);
    ok &= img;
    ok &= walk_probe();
    if ok {
        uart::write_str_raw("xo: ok text=el0-xo rodata=app-hdr el0-read=fault sys-read=deny slots=5\n");
    }
    ok
}

/// Serial proof (ADR-097).
#[allow(dead_code)]
pub fn observe_probe() -> bool {
    run_probe(true)
}

#[cfg(test)]
#[test_case]
fn el0_app_text_is_execute_only() {
    assert!(run_probe(false), "app text must be EL0 execute-only with rodata on the app-hdr page");
}
