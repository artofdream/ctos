//! ADR-096: task-to-task EL0 isolation probe (one core, QEMU TCG).
//!
//! Two EL0 tasks, two address spaces:
//! - **task A**: user TTBR0 (`L1_USER`, ASID 1), the TTBR0 every loaded app
//!   uses today;
//! - **task B**: ASID-B TTBR0 (`L1_ASID_B`, ASID 2), its own map-window L3.
//!
//! Both use the same VA for their private data page: `T2T_VA`
//! (`LOADER_STACK_VA`, the allowlisted `user-stack` slot), backed by
//! different pool frames, `nG` so TLB entries are ASID-tagged. Both code
//! pages sit at `EL0_PAGE` (the allowlisted `app-text` slot); no probe page
//! leaves the ADR-092 allowlist and all are unmapped before the steady walk.
//!
//! Round 1 (A has nothing at `T2T_VA`): B stores and reloads its sentinel;
//! then, with no TLBI of `T2T_VA` in between, A loads and stores `T2T_VA`.
//! Both must take a lower-EL translation fault (FAR = `T2T_VA`), A must not
//! see B's sentinel, and B's frame must be unchanged. A walk of A's TTBR0
//! must find no leaf at all for B's frame.
//! Round 2 (same VA, A has its own frame): A must read its own sentinel,
//! A's store must land only in A's frame, and B must still read its own.
//!
//! Fail-closed: any leak prints `t2t: leak …`, anything unexpected prints
//! `t2t: bad …`, and `t2t: ok` is withheld. The `t2t-leak-probe` build maps
//! B's frame into A's TTBR0 (a shared-frame bug) and must be caught.
//!
//! Limits: one core; QEMU TCG, where a TTBR0 write that changes the ASID
//! flushes QEMU's own software TLB, so the "no TLBI after B" step shows the
//! kernel does not rely on a flush, not how a hardware ASID-tagged TLB
//! behaves; two kernel-built tasks, not two loaded apps; not a side-channel
//! probe. Evidence next to the ADR-091 sentence, not part of it.

use core::fmt::Write;
use core::hint::black_box;

use crate::exception;
use crate::frame;
use crate::paging::{self, T2tRoot};
use crate::uart;

/// Shared private-data VA (allowlisted `user-stack` slot).
pub const T2T_VA: u64 = paging::LOADER_STACK_VA;
/// Code VA for both tasks (allowlisted `app-text` slot).
const CODE_VA: u64 = paging::EL0_PAGE;

const SENT_A: u64 = 0x7a5c_a0a0;
const SENT_A2: u64 = 0x7a5c_a2a2;
const SENT_B: u64 = 0x7a5c_b0b0;
/// What A tries to store into B's page in round 1.
const SENT_EVIL: u64 = 0x7a5c_eeee;

const LDR_X1_X0: u32 = 0xF940_0001;
const STR_X1_X0: u32 = 0xF900_0001;
const BRK0: u32 = 0xD420_0000;
const EC_DABORT_LOWER: u64 = 0x24;
const EC_BRK: u64 = 0x3C;

fn movz(rd: u32, imm: u16, hw: u32) -> u32 {
    0xD280_0000 | (hw << 21) | ((imm as u32) << 5) | rd
}

fn movk(rd: u32, imm: u16, hw: u32) -> u32 {
    0xF280_0000 | (hw << 21) | ((imm as u32) << 5) | rd
}

/// `MOVZ X1, #0; LDR X1, [X0]; BRK #0`. The load is at +4.
fn code_read() -> [u32; 5] {
    [movz(1, 0, 0), LDR_X1_X0, BRK0, BRK0, BRK0]
}

/// `X1 = val; STR X1, [X0]; LDR X1, [X0]; BRK #0`. The store is at +8.
fn code_write(val: u64) -> [u32; 5] {
    [
        movz(1, (val & 0xffff) as u16, 0),
        movk(1, ((val >> 16) & 0xffff) as u16, 1),
        STR_X1_X0,
        LDR_X1_X0,
        BRK0,
    ]
}

fn frame_zeroed() -> Option<u64> {
    let pa = frame::alloc()?;
    unsafe { core::ptr::write_bytes(paging::frame_cpu_va(pa) as *mut u8, 0, 4096) };
    Some(pa)
}

fn frame_read(pa: u64) -> u64 {
    unsafe { core::ptr::read_volatile(paging::frame_cpu_va(pa) as *const u64) }
}

fn frame_write(pa: u64, v: u64) {
    unsafe { core::ptr::write_volatile(paging::frame_cpu_va(pa) as *mut u64, v) };
}

/// Write code through the frame's TTBR1 alias, then make it visible to the
/// instruction side (one cache line).
fn load_code(pa: u64, code: &[u32; 5]) {
    let alias = paging::frame_cpu_va(pa);
    for (i, w) in code.iter().enumerate() {
        unsafe { core::ptr::write_volatile((alias as *mut u32).add(i), *w) };
    }
    unsafe {
        core::arch::asm!(
            "dc cvau, {x}",
            "dsb ish",
            "ic iallu",
            "dsb ish",
            "isb",
            x = in(reg) alias,
            options(nostack, preserves_flags),
        );
    }
}

/// One captured EL0 trip: `(taken, esr, far, elr, x1)`.
type Cap = (bool, u64, u64, u64, u64);

/// Task B: code page already mapped at `CODE_VA` in the ASID-B L3.
fn run_b(code_pa: u64, code: &[u32; 5]) -> Cap {
    load_code(code_pa, code);
    exception::arm_el0_capture();
    unsafe {
        exception::eret_to_el0_masked_ttbr(black_box(CODE_VA), T2T_VA, CODE_VA + 4096, paging::asid_b_ttbr0());
    }
    exception::el0_capture_result()
}

/// Task A: map the code page at `CODE_VA` in the shared window (user
/// TTBR0) for this trip only. Map/unmap invalidate `CODE_VA` only, never
/// `T2T_VA`, so B's ASID-2 entry for `T2T_VA` is not flushed by the kernel.
fn run_a(code_pa: u64, code: &[u32; 5]) -> Option<Cap> {
    load_code(code_pa, code);
    if !paging::map_el0_exec(CODE_VA, code_pa) {
        return None;
    }
    exception::arm_el0_capture();
    unsafe {
        exception::eret_to_el0_masked(black_box(CODE_VA), T2T_VA, CODE_VA + 4096);
    }
    let cap = exception::el0_capture_result();
    let _ = paging::unmap_page(CODE_VA);
    Some(cap)
}

fn ec(esr: u64) -> u64 {
    (esr >> 26) & 0x3f
}

fn wnr(esr: u64) -> u64 {
    (esr >> 6) & 1
}

/// Translation-fault level 0..=3, or `None`.
fn trans_level(esr: u64) -> Option<u64> {
    let dfsc = esr & 0x3f;
    if (0x04..=0x07).contains(&dfsc) {
        Some(dfsc - 0x04)
    } else {
        None
    }
}

/// B's own trip: BRK taken with x1 = `want` and the frame holding `want`.
fn b_ok(cap: Cap, want: u64, b_pa: u64) -> bool {
    let (taken, esr, _far, elr, x1) = cap;
    taken && ec(esr) == EC_BRK && x1 == want && frame_read(b_pa) == want && elr >= CODE_VA && elr < CODE_VA + 32
}

struct Frames {
    a_code: u64,
    b_code: u64,
    a_data: u64,
    b_data: u64,
}

fn cleanup(f: &Frames) {
    paging::switch_ttbr0_no_tlbi(paging::kernel_ttbr0());
    let _ = paging::unmap_page(CODE_VA);
    let _ = paging::unmap_page(T2T_VA);
    let _ = paging::unmap_asid_b_page(CODE_VA);
    let _ = paging::unmap_asid_b_page(T2T_VA);
    // TLBI VAAE1: every ASID, incl. B's nG entries.
    paging::invalidate_va(CODE_VA);
    paging::invalidate_va(T2T_VA);
    frame::free(f.a_code);
    frame::free(f.b_code);
    frame::free(f.a_data);
    frame::free(f.b_data);
}

/// Round 1: A has no mapping at `T2T_VA`.
fn round1(f: &Frames) -> bool {
    let mut w = uart::raw();
    let mut ok = true;
    #[cfg(feature = "t2t-leak-probe")]
    if paging::map_el0_rw_ng(T2T_VA, f.b_data) {
        uart::write_str_raw("t2t: leak-probe planted b-frame in a-ttbr0 va=0x80007000\n");
    }
    // Walk: A's TTBR0 must have no leaf at all for B's frame; B's TTBR0 has
    // exactly one EL0-reachable leaf for it (control: the walk sees pages).
    let (u_refs, u_el0) = paging::pa_refs(T2tRoot::User, f.b_data);
    let (_b_refs, b_el0) = paging::pa_refs(T2tRoot::AsidB, f.b_data);
    if u_refs != 0 || u_el0 != 0 {
        let _ = writeln!(w, "t2t: leak walk b-frame u-refs={} u-el0={}", u_refs, u_el0);
        ok = false;
    } else if b_el0 != 1 {
        let _ = writeln!(w, "t2t: bad walk b-frame a-el0={}", b_el0);
        return false;
    } else {
        let _ = writeln!(w, "t2t: walk b-frame u-refs=0 u-el0=0 a-el0=1");
    }

    // B stores and reloads its sentinel (ASID 2).
    let cap = run_b(f.b_code, &code_write(SENT_B));
    if !b_ok(cap, SENT_B, f.b_data) {
        let _ = writeln!(w, "t2t: bad b-own esr={:#x} x1={:#x} frame={:#x}", cap.1, cap.4, frame_read(f.b_data));
        return false;
    }
    let _ = writeln!(w, "t2t: b-own ok asid=2 va={:#x} val={:#x}", T2T_VA, SENT_B);

    // A loads T2T_VA right after B ran (no TLBI of T2T_VA in between).
    let Some((taken, esr, far, elr, x1)) = run_a(f.a_code, &code_read()) else {
        uart::write_str_raw("t2t: bad a-read map\n");
        return false;
    };
    if taken && x1 == SENT_B {
        let _ = writeln!(w, "t2t: leak read asid=1 va={:#x} got={:#x}", T2T_VA, x1);
        ok = false;
    } else if !taken
        || ec(esr) != EC_DABORT_LOWER
        || wnr(esr) != 0
        || trans_level(esr).is_none()
        || far != T2T_VA
        || elr != CODE_VA + 4
        || x1 != 0
    {
        let _ = writeln!(w, "t2t: bad a-read taken={} esr={:#x} far={:#x} elr={:#x} x1={:#x}", taken, esr, far, elr, x1);
        return false;
    } else {
        let _ = writeln!(
            w,
            "t2t: a-read fault asid=1 va={:#x} esr={:#x} far={:#x} ec=0x24 wnr=0 dfsc=trans-l{} got={:#x} after-b no-tlbi",
            T2T_VA,
            esr,
            far,
            trans_level(esr).unwrap_or(9),
            x1
        );
    }

    // A tries to store into T2T_VA; B's frame must stay SENT_B.
    let Some((taken, esr, far, elr, _x1)) = run_a(f.a_code, &code_write(SENT_EVIL)) else {
        uart::write_str_raw("t2t: bad a-write map\n");
        return false;
    };
    let b_now = frame_read(f.b_data);
    if b_now != SENT_B {
        let _ = writeln!(w, "t2t: leak write asid=1 va={:#x} b={:#x}", T2T_VA, b_now);
        ok = false;
    } else if !taken
        || ec(esr) != EC_DABORT_LOWER
        || wnr(esr) != 1
        || trans_level(esr).is_none()
        || far != T2T_VA
        || elr != CODE_VA + 8
    {
        let _ = writeln!(w, "t2t: bad a-write taken={} esr={:#x} far={:#x} elr={:#x}", taken, esr, far, elr);
        return false;
    } else {
        let _ = writeln!(
            w,
            "t2t: a-write fault asid=1 va={:#x} esr={:#x} far={:#x} ec=0x24 wnr=1 dfsc=trans-l{} b-intact=true",
            T2T_VA,
            esr,
            far,
            trans_level(esr).unwrap_or(9)
        );
    }
    #[cfg(feature = "t2t-leak-probe")]
    {
        let _ = paging::unmap_page(T2T_VA);
    }
    ok
}

/// Round 2: same VA, A has its own frame (nG) in the user TTBR0.
fn round2(f: &Frames) -> bool {
    let mut w = uart::raw();
    frame_write(f.a_data, SENT_A);
    if !paging::map_el0_rw_ng(T2T_VA, f.a_data) {
        uart::write_str_raw("t2t: bad same-va map\n");
        return false;
    }
    let (ua_refs, ua_el0) = paging::pa_refs(T2tRoot::User, f.a_data);
    let (_ba_refs, ba_el0) = paging::pa_refs(T2tRoot::AsidB, f.a_data);
    let (ub_refs, ub_el0) = paging::pa_refs(T2tRoot::User, f.b_data);
    let (_bb_refs, bb_el0) = paging::pa_refs(T2tRoot::AsidB, f.b_data);
    if ba_el0 != 0 || ub_refs != 0 || ub_el0 != 0 {
        let _ = writeln!(w, "t2t: leak walk same-va a-frame a-el0={} b-frame u-refs={} u-el0={}", ba_el0, ub_refs, ub_el0);
        return false;
    }
    if ua_el0 != 1 || ua_refs == 0 || bb_el0 != 1 {
        let _ = writeln!(w, "t2t: bad walk same-va u-el0={} a-el0={}", ua_el0, bb_el0);
        return false;
    }
    let _ = writeln!(w, "t2t: walk same-va a-frame u-el0=1 a-el0=0 b-frame u-el0=0 a-el0=1");

    // B runs again first, so its ASID-2 entry for T2T_VA is fresh.
    let cap = run_b(f.b_code, &code_write(SENT_B));
    if !b_ok(cap, SENT_B, f.b_data) || frame_read(f.a_data) != SENT_A {
        let _ = writeln!(w, "t2t: bad same-va b-first x1={:#x} a={:#x}", cap.4, frame_read(f.a_data));
        return false;
    }
    // A reads the same VA: must see its own sentinel, never B's.
    let Some((taken, esr, _far, _elr, x1)) = run_a(f.a_code, &code_read()) else {
        return false;
    };
    if x1 == SENT_B {
        let _ = writeln!(w, "t2t: leak read same-va asid=1 got={:#x}", x1);
        return false;
    }
    if !taken || ec(esr) != EC_BRK || x1 != SENT_A {
        let _ = writeln!(w, "t2t: bad same-va a-read taken={} esr={:#x} x1={:#x}", taken, esr, x1);
        return false;
    }
    let _ = writeln!(w, "t2t: same-va a-sees-own asid=1 got={:#x} b={:#x}", x1, frame_read(f.b_data));
    // A writes the same VA: lands in A's frame only.
    let Some((taken, esr, _far, _elr, x1)) = run_a(f.a_code, &code_write(SENT_A2)) else {
        return false;
    };
    let b_now = frame_read(f.b_data);
    if b_now != SENT_B {
        let _ = writeln!(w, "t2t: leak write same-va asid=1 b={:#x}", b_now);
        return false;
    }
    if !taken || ec(esr) != EC_BRK || x1 != SENT_A2 || frame_read(f.a_data) != SENT_A2 {
        let _ = writeln!(w, "t2t: bad same-va a-write taken={} esr={:#x} x1={:#x}", taken, esr, x1);
        return false;
    }
    let _ = writeln!(w, "t2t: same-va a-write-own got={:#x} b-intact=true", x1);
    // B reads again after A's trips: its own value, and A's frame untouched.
    let cap = run_b(f.b_code, &code_read());
    let a_now = frame_read(f.a_data);
    if cap.4 == SENT_A2 || cap.4 == SENT_A {
        let _ = writeln!(w, "t2t: leak read same-va asid=2 got={:#x}", cap.4);
        return false;
    }
    if !b_ok(cap, SENT_B, f.b_data) || a_now != SENT_A2 {
        let _ = writeln!(w, "t2t: bad same-va b-after x1={:#x} a={:#x}", cap.4, a_now);
        return false;
    }
    let _ = writeln!(w, "t2t: same-va b-sees-own asid=2 got={:#x} a-intact=true", cap.4);
    true
}

fn run_probe() -> bool {
    if !paging::mmu_enabled() || !paging::user_map_ready() || crate::el0::is_active() {
        return false;
    }
    // A and B must really be distinct address spaces with distinct ASIDs.
    let ua = paging::user_ttbr0();
    let ub = paging::asid_b_ttbr0();
    if ua >> 48 != paging::USER_ASID || ub >> 48 != paging::ASID_ISO_B || ua & !(0xffff << 48) == ub & !(0xffff << 48) {
        uart::write_str_raw("t2t: bad roots\n");
        return false;
    }
    if !paging::prepare_asid_b_tables() {
        return false;
    }
    let (Some(a_code), Some(b_code), Some(a_data), Some(b_data)) =
        (frame_zeroed(), frame_zeroed(), frame_zeroed(), frame_zeroed())
    else {
        return false;
    };
    let f = Frames { a_code, b_code, a_data, b_data };
    // The window slots must be free in both roots before we start.
    if paging::l3_entry(CODE_VA).unwrap_or(1) != 0
        || paging::l3_entry(T2T_VA).unwrap_or(1) != 0
        || paging::asid_b_leaf(CODE_VA).unwrap_or(1) != 0
        || paging::asid_b_leaf(T2T_VA).unwrap_or(1) != 0
    {
        uart::write_str_raw("t2t: bad slots busy\n");
        cleanup(&f);
        return false;
    }
    if !paging::map_asid_b_el0(CODE_VA, b_code, true) || !paging::map_asid_b_el0(T2T_VA, b_data, false) {
        cleanup(&f);
        return false;
    }
    let b_leaf = paging::asid_b_leaf(T2T_VA).unwrap_or(0);
    if !paging::desc_is_ng(b_leaf) {
        uart::write_str_raw("t2t: bad b-leaf not nG\n");
        cleanup(&f);
        return false;
    }
    paging::invalidate_va(CODE_VA);
    paging::invalidate_va(T2T_VA);
    let ok = round1(&f) && round2(&f);
    cleanup(&f);
    if ok {
        let mut w = uart::raw();
        let _ = writeln!(w, "t2t: ok asid-a=1 asid-b=2 va={:#x} read,write,same-va", T2T_VA);
    }
    ok
}

/// Serial proof (ADR-096): task A cannot read or write task B's page.
#[allow(dead_code)]
pub fn observe_probe() -> bool {
    run_probe()
}

#[cfg(test)]
#[test_case]
fn el0_task_cannot_read_or_write_other_task() {
    assert!(run_probe(), "task A (ASID 1) must not reach task B's page (ASID 2)");
}
