//! ADR-105 (S5, `smp2-probe`): a locked run queue shared by both cores.
//!
//! Tasks are small integers. A core takes the next runnable task under the
//! lock ([`pick`]), marks itself as its owner and records it in its
//! `PerCpu::current`; it gives it back with [`put`]. A task owned by one
//! core can never be picked by the other: `pick` refuses an owned task, and
//! an owner mismatch on `put` is counted as a violation. Round-robin, no
//! priorities, no preemption (each trip runs one EL0 entry to its exit).

use core::sync::atomic::{AtomicU64, Ordering};

use spin::Mutex;

pub const MAX_TASKS: usize = 8;

#[derive(Clone, Copy)]
struct Entry {
    /// 0 = free slot, 1 = runnable / parked, 2 = finished.
    state: u8,
    /// Owning core + 1 (0 = none).
    owner: u8,
}

struct State {
    entries: [Entry; MAX_TASKS],
    next: usize,
    n: usize,
}

static STATE: Mutex<State> = Mutex::new(State {
    entries: [Entry { state: 0, owner: 0 }; MAX_TASKS],
    next: 0,
    n: 0,
});

static VIOLATIONS: AtomicU64 = AtomicU64::new(0);

/// Register a new task. Returns its id, or `None` if the table is full.
pub fn spawn() -> Option<usize> {
    let mut s = STATE.lock();
    for i in 0..MAX_TASKS {
        if s.entries[i].state == 0 {
            s.entries[i] = Entry { state: 1, owner: 0 };
            s.n += 1;
            return Some(i);
        }
    }
    None
}

/// Pick the next runnable unowned task. Records it in this core's
/// `PerCpu::current` (id + 1). Returns `None` when every living task is
/// owned by another core or finished.
pub fn pick() -> Option<usize> {
    let cpu = crate::percpu::id();
    let mut s = STATE.lock();
    if s.n == 0 {
        return None;
    }
    for k in 0..MAX_TASKS {
        let i = (s.next + k) % MAX_TASKS;
        let e = &mut s.entries[i];
        if e.state == 1 && e.owner == 0 {
            e.owner = (cpu as u8) + 1;
            s.next = (i + 1) % MAX_TASKS;
            drop(s);
            if let Some(b) = crate::percpu::this() {
                b.current.store((i as u64) + 1, Ordering::SeqCst);
            }
            return Some(i);
        }
    }
    None
}

/// Return a task this core owns. `done` finishes it; otherwise it becomes
/// runnable again. Owner mismatch increments [`violations`].
pub fn put(id: usize, done: bool) {
    let cpu = crate::percpu::id();
    let mut s = STATE.lock();
    if id >= MAX_TASKS {
        VIOLATIONS.fetch_add(1, Ordering::SeqCst);
        return;
    }
    let e = &mut s.entries[id];
    if e.owner != (cpu as u8) + 1 {
        VIOLATIONS.fetch_add(1, Ordering::SeqCst);
        return;
    }
    e.owner = 0;
    e.state = if done { 2 } else { 1 };
    if done {
        s.n = s.n.saturating_sub(1);
    }
    drop(s);
    if let Some(b) = crate::percpu::this() {
        b.current.store(0, Ordering::SeqCst);
    }
}

/// Owner mismatches so far (must stay 0).
pub fn violations() -> u64 {
    VIOLATIONS.load(Ordering::SeqCst)
}

/// Reset for a new probe.
pub fn reset() {
    let mut s = STATE.lock();
    *s = State {
        entries: [Entry { state: 0, owner: 0 }; MAX_TASKS],
        next: 0,
        n: 0,
    };
    VIOLATIONS.store(0, Ordering::SeqCst);
}
