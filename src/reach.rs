//! ADR-092 (G1): fail-closed EL0-reachability walk reporting.
//!
//! Three views of the same rule set (`paging::el0_reach`):
//! 1. **live**: armed before the hello-libctos loader run; taken on the app's
//!    first `SYS_YIELD` while it stands at EL0 with every user page mapped;
//! 2. **steady**: after the boot probes; nothing may be EL0-reachable;
//! 3. **negative**: four planted leaves (kernel-PA, off-allowlist VA, TTBR1,
//!    EL0 W+X) must each be caught, then a clean walk.
//! Not “EL0 isolated”; a walk of this kernel's own tables on QEMU TCG.

use core::fmt::Write;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use crate::frame;
use crate::paging::{self, El0Reach, ReachPlant};
use crate::uart;

static LIVE_ARMED: AtomicBool = AtomicBool::new(false);
static LIVE_TAKEN: AtomicBool = AtomicBool::new(false);
static LIVE_PAGES: [AtomicU32; 4] = [AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0)];
static LIVE_LEAKS: AtomicU32 = AtomicU32::new(u32::MAX);

/// Kernel `.data` page used as the kernel-PA plant target (never EL0-run).
static REACH_KERNEL_PAGE_BAIT: AtomicU32 = AtomicU32::new(0x5eac_4ba1);

fn total(p: &[u32; 4]) -> u32 {
    p.iter().sum()
}

fn print_counts(tag: &str, r: &El0Reach) {
    let mut w = uart::raw();
    let _ = writeln!(
        w,
        "el0-reach: {} k={} u={} a={} h={} pages={} leaks={}",
        tag,
        r.pages[0],
        r.pages[1],
        r.pages[2],
        r.pages[3],
        total(&r.pages),
        r.leaks
    );
}

/// Arm the live walk for the next `SYS_YIELD` from EL0.
pub fn arm_live() {
    LIVE_TAKEN.store(false, Ordering::SeqCst);
    LIVE_LEAKS.store(u32::MAX, Ordering::SeqCst);
    LIVE_ARMED.store(true, Ordering::SeqCst);
}

/// Called from the `SYS_YIELD` handler (EL1, user standing at EL0).
pub fn on_yield() {
    if !LIVE_ARMED.swap(false, Ordering::SeqCst) {
        return;
    }
    let r = paging::el0_reach(true);
    for i in 0..4 {
        LIVE_PAGES[i].store(r.pages[i], Ordering::SeqCst);
    }
    LIVE_LEAKS.store(r.leaks, Ordering::SeqCst);
    LIVE_TAKEN.store(true, Ordering::SeqCst);
    print_counts("live", &r);
}

fn kernel_bait_pa() -> u64 {
    paging::identity_pa(core::ptr::addr_of!(REACH_KERNEL_PAGE_BAIT) as u64) & !0xfff
}

fn plant_name(k: ReachPlant) -> &'static str {
    match k {
        ReachPlant::KernelPa => "kernel-pa",
        ReachPlant::Va => "va",
        ReachPlant::Ttbr1 => "ttbr1",
        ReachPlant::El0Wx => "el0-wx",
    }
}

fn run_negative() -> bool {
    let Some(pool_pa) = frame::alloc() else {
        return false;
    };
    let kpa = kernel_bait_pa();
    let kinds = [ReachPlant::KernelPa, ReachPlant::Va, ReachPlant::Ttbr1, ReachPlant::El0Wx];
    let mut ok = true;
    for kind in kinds {
        let name = plant_name(kind);
        let mut w = uart::raw();
        match paging::el0_reach_plant_probe(kind, kpa, pool_pa) {
            None => {
                let _ = writeln!(w, "el0-reach: neg {} missed slot-busy", name);
                ok = false;
            }
            Some((va, r)) => {
                let roots: [&str; 4] = paging::REACH_ROOT_TAGS;
                let mut hit = [false; 4];
                hit.copy_from_slice(&r.query_hit);
                let any = hit.iter().any(|&h| h);
                // Window plants sit in the L3 shared by kernel + user TTBR0.
                let want: [bool; 4] = match kind {
                    ReachPlant::Ttbr1 => [false, false, false, true],
                    _ => [true, true, false, false],
                };
                if !any || hit != want || r.query_why != name {
                    let _ = writeln!(w, "el0-reach: neg {} missed va={:#x} why={}", name, va, r.query_why);
                    ok = false;
                    continue;
                }
                let _ = write!(w, "el0-reach: neg {} caught va={:#x} roots=", name, va);
                let mut first = true;
                for i in 0..4 {
                    if hit[i] {
                        let _ = write!(w, "{}{}", if first { "" } else { "," }, roots[i]);
                        first = false;
                    }
                }
                let _ = writeln!(w, " why={}", r.query_why);
            }
        }
    }
    frame::free(pool_pa);
    let clean = paging::el0_reach(false);
    if clean.leaks != 0 || total(&clean.pages) != 0 {
        let mut w = uart::raw();
        let _ = writeln!(w, "el0-reach: neg unclean leaks={} pages={}", clean.leaks, total(&clean.pages));
        return false;
    }
    if ok {
        uart::write_str_raw("el0-reach: neg clean\n");
    }
    ok
}

fn run_probe() -> bool {
    if !LIVE_TAKEN.load(Ordering::SeqCst) {
        uart::write_str_raw("el0-reach: live missed\n");
        return false;
    }
    let live_u = LIVE_PAGES[1].load(Ordering::SeqCst);
    let live_leaks = LIVE_LEAKS.load(Ordering::SeqCst);
    if live_leaks != 0 || live_u == 0 {
        let mut w = uart::raw();
        let _ = writeln!(w, "el0-reach: live bad u={} leaks={}", live_u, live_leaks);
        return false;
    }
    #[cfg(feature = "reach-leak-probe")]
    if paging::plant_persistent_reach_leak(kernel_bait_pa()) {
        uart::write_str_raw("el0-reach: leak-probe planted va=0x80007000\n");
    }
    let steady = paging::el0_reach(true);
    print_counts("steady", &steady);
    if steady.leaks != 0 || total(&steady.pages) != 0 {
        return false;
    }
    if !run_negative() {
        return false;
    }
    let mut w = uart::raw();
    let _ = write!(w, "el0-reach: ok allow=");
    for (i, &(lo, hi, name)) in paging::EL0_ALLOW.iter().enumerate() {
        let _ = write!(w, "{}{}[{:#x},{:#x})", if i == 0 { "" } else { "," }, name, lo, hi);
    }
    let _ = writeln!(w);
    true
}

/// Serial proof (ADR-092): live + steady + negative EL0-reachability walks.
#[allow(dead_code)]
pub fn observe_probe() -> bool {
    run_probe()
}

/// ADR-092 (G1): at rest nothing is EL0-reachable; each planted leaf is
/// caught under the right rule and root; the walk is clean afterwards.
#[cfg(test)]
#[test_case]
fn el0_reach_rest_clean_and_plants_caught() {
    let rest = paging::el0_reach(false);
    assert_eq!(rest.leaks, 0, "EL0-reachable leaf breaks a rule at rest");
    assert_eq!(total(&rest.pages), 0, "no EL0-reachable page expected at rest");
    assert!(run_negative(), "a planted EL0-reachable leaf was not caught");
}
