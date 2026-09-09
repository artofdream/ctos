//! Cooperative round-robin tasks on EL1 (FR-11 / M9).
//!
//! Tasks yield by saving AAPCS64 callee-saved GPRs on their own stack
//! and loading the next task's saved SP. The bootstrap thread (slot 0)
//! keeps the linker `SP_EL0` stack. Worker stacks are heap `Box`es.
//! Not preemptive, not SMP, not EL0. See ADR-010.

use alloc::vec::Vec;
use core::fmt::Write;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use spin::Mutex;

use crate::uart;

/// Two workers + idle. Enough for the FR-11 probe.
const MAX_TASKS: usize = 3;
/// 8 KiB per worker. Allocated on the heap so we do not smash `SP_EL0`.
pub const TASK_STACK_SIZE: usize = 8192;
const CALLEE_SAVED_BYTES: usize = 12 * 8;
const YIELD_BOUND: usize = 16;

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Unused,
    Ready,
    Running,
    Done,
}

struct Task {
    /// SP after the callee-saved push (or 0 until the first save).
    sp: u64,
    state: State,
    stack_lo: u64,
    stack_hi: u64,
    entry: Option<fn()>,
    stack: Option<Vec<u8>>,
}

impl Task {
    const fn empty() -> Self {
        Self {
            sp: 0,
            state: State::Unused,
            stack_lo: 0,
            stack_hi: 0,
            entry: None,
            stack: None,
        }
    }
}

struct Scheduler {
    tasks: [Task; MAX_TASKS],
    current: usize,
}

impl Scheduler {
    const fn new() -> Self {
        Self {
            tasks: [Task::empty(), Task::empty(), Task::empty()],
            current: 0,
        }
    }

    fn init_idle(&mut self) {
        self.tasks[0].state = State::Running;
        self.tasks[0].stack_lo = crate::exception::thread_stack_bottom();
        self.tasks[0].stack_hi = crate::exception::thread_stack_top();
        self.current = 0;
    }

    fn spawn(&mut self, entry: fn()) -> Option<usize> {
        let slot = (1..MAX_TASKS).find(|&i| self.tasks[i].state == State::Unused)?;
        let mut stack = Vec::new();
        stack.resize(TASK_STACK_SIZE, 0);
        let base = stack.as_mut_ptr() as u64;
        let top = (base + TASK_STACK_SIZE as u64) & !0xf;
        let frame = top.saturating_sub(CALLEE_SAVED_BYTES as u64);
        if frame < base || frame & 0xf != 0 {
            return None;
        }
        unsafe {
            write_initial_frame(frame, task_trampoline as u64);
        }
        self.tasks[slot] = Task {
            sp: frame,
            state: State::Ready,
            stack_lo: base,
            stack_hi: top,
            entry: Some(entry),
            stack: Some(stack),
        };
        Some(slot)
    }

    fn reset_workers(&mut self) {
        if self.current != 0 {
            return;
        }
        for t in self.tasks.iter_mut().skip(1) {
            *t = Task::empty();
        }
    }

    fn pick_next(&self) -> Option<usize> {
        for i in 1..=MAX_TASKS {
            let idx = (self.current + i) % MAX_TASKS;
            if self.tasks[idx].state == State::Ready {
                return Some(idx);
            }
        }
        None
    }

    /// Unlock before calling `context_switch`. Pointers stay in the static.
    fn prepare_switch(&mut self) -> Option<(*mut u64, u64)> {
        let next = self.pick_next()?;
        let cur = self.current;
        if self.tasks[cur].state == State::Running {
            self.tasks[cur].state = State::Ready;
        }
        self.tasks[next].state = State::Running;
        self.current = next;
        let old_sp = core::ptr::addr_of_mut!(self.tasks[cur].sp);
        let new_sp = self.tasks[next].sp;
        Some((old_sp, new_sp))
    }
}

static SCHED: Mutex<Scheduler> = Mutex::new(Scheduler::new());
static TASK_A: AtomicBool = AtomicBool::new(false);
static TASK_B: AtomicBool = AtomicBool::new(false);
static TASK_A_SP: AtomicU64 = AtomicU64::new(0);
static TASK_B_SP: AtomicU64 = AtomicU64::new(0);
static SWITCHES: AtomicU64 = AtomicU64::new(0);

unsafe extern "C" {
    fn context_switch(old_sp: *mut u64, new_sp: u64);
}

core::arch::global_asm!(
    r#"
    .section .text
    .align 2
    .global context_switch
context_switch:
    // x0 = &current.sp, x1 = next.sp
    // Layout at SP must match write_initial_frame / CalleeSaved.
    stp x29, x30, [sp, #-16]!
    stp x27, x28, [sp, #-16]!
    stp x25, x26, [sp, #-16]!
    stp x23, x24, [sp, #-16]!
    stp x21, x22, [sp, #-16]!
    stp x19, x20, [sp, #-16]!
    mov x2, sp
    str x2, [x0]
    mov sp, x1
    ldp x19, x20, [sp], #16
    ldp x21, x22, [sp], #16
    ldp x23, x24, [sp], #16
    ldp x25, x26, [sp], #16
    ldp x27, x28, [sp], #16
    ldp x29, x30, [sp], #16
    ret
    "#
);

/// Memory at `frame` as seen when `context_switch` restores (last push first).
///
/// Push order is x29/x30 first … x19/x20 last, so SP points at x19/x20.
#[repr(C)]
struct CalleeSaved {
    x19: u64,
    x20: u64,
    x21: u64,
    x22: u64,
    x23: u64,
    x24: u64,
    x25: u64,
    x26: u64,
    x27: u64,
    x28: u64,
    x29: u64,
    x30: u64,
}

const _: () = assert!(core::mem::size_of::<CalleeSaved>() == CALLEE_SAVED_BYTES);

unsafe fn write_initial_frame(frame: u64, trampoline: u64) {
    let ctx = frame as *mut CalleeSaved;
    ctx.write(CalleeSaved {
        x19: 0,
        x20: 0,
        x21: 0,
        x22: 0,
        x23: 0,
        x24: 0,
        x25: 0,
        x26: 0,
        x27: 0,
        x28: 0,
        x29: 0,
        x30: trampoline,
    });
}

extern "C" fn task_trampoline() {
    let entry = {
        let mut s = SCHED.lock();
        s.tasks[s.current].entry.take()
    };
    if let Some(entry) = entry {
        entry();
    }
    {
        let mut s = SCHED.lock();
        let cur = s.current;
        if s.tasks[cur].state != State::Unused {
            s.tasks[cur].state = State::Done;
        }
    }
    loop {
        yield_now();
    }
}

/// Install idle slot 0. Heap must already be up.
pub fn init() {
    SCHED.lock().init_idle();
}

/// Cooperative yield. No-op if no other Ready task exists.
pub fn yield_now() {
    let (old_sp, new_sp) = {
        let mut s = SCHED.lock();
        match s.prepare_switch() {
            Some(pair) => pair,
            None => return,
        }
    };
    SWITCHES.fetch_add(1, Ordering::SeqCst);
    unsafe {
        context_switch(old_sp, new_sp);
    }
}

fn spawn(entry: fn()) -> Option<usize> {
    SCHED.lock().spawn(entry)
}

fn reset_workers() {
    SCHED.lock().reset_workers();
}

fn current_sp() -> u64 {
    let sp: u64;
    unsafe {
        core::arch::asm!("mov {s}, sp", s = out(reg) sp);
    }
    sp
}

fn task_a() {
    TASK_A_SP.store(current_sp(), Ordering::SeqCst);
    uart::write_str_raw("sched: task a\n");
    TASK_A.store(true, Ordering::SeqCst);
}

fn task_b() {
    TASK_B_SP.store(current_sp(), Ordering::SeqCst);
    uart::write_str_raw("sched: task b\n");
    TASK_B.store(true, Ordering::SeqCst);
}

fn run_two_tasks() -> bool {
    if !crate::heap::is_ready() {
        return false;
    }
    reset_workers();
    TASK_A.store(false, Ordering::SeqCst);
    TASK_B.store(false, Ordering::SeqCst);
    TASK_A_SP.store(0, Ordering::SeqCst);
    TASK_B_SP.store(0, Ordering::SeqCst);
    if spawn(task_a).is_none() || spawn(task_b).is_none() {
        return false;
    }
    for _ in 0..YIELD_BOUND {
        if TASK_A.load(Ordering::SeqCst) && TASK_B.load(Ordering::SeqCst) {
            break;
        }
        yield_now();
    }
    TASK_A.load(Ordering::SeqCst) && TASK_B.load(Ordering::SeqCst)
}

/// Serial proof: both workers run on heap stacks, then `sched: ok`.
#[allow(dead_code)] // hello kernel only; cargo test uses the cases below.
pub fn observe_probe() -> bool {
    if !run_two_tasks() {
        return false;
    }
    let a_sp = TASK_A_SP.load(Ordering::SeqCst);
    let b_sp = TASK_B_SP.load(Ordering::SeqCst);
    let (a_lo, a_hi, b_lo, b_hi) = {
        let s = SCHED.lock();
        (
            s.tasks[1].stack_lo,
            s.tasks[1].stack_hi,
            s.tasks[2].stack_lo,
            s.tasks[2].stack_hi,
        )
    };
    if a_sp <= a_lo || a_sp > a_hi || b_sp <= b_lo || b_sp > b_hi || a_sp == b_sp {
        return false;
    }
    let mut w = uart::raw();
    let _ = writeln!(w, "sched: ok");
    true
}

#[cfg(test)]
#[test_case]
fn two_tasks_run_on_distinct_heap_stacks() {
    assert!(run_two_tasks(), "both workers must run");
    let a_sp = TASK_A_SP.load(Ordering::SeqCst);
    let b_sp = TASK_B_SP.load(Ordering::SeqCst);
    let (a_lo, a_hi, b_lo, b_hi, idle_lo, idle_hi) = {
        let s = SCHED.lock();
        (
            s.tasks[1].stack_lo,
            s.tasks[1].stack_hi,
            s.tasks[2].stack_lo,
            s.tasks[2].stack_hi,
            s.tasks[0].stack_lo,
            s.tasks[0].stack_hi,
        )
    };
    assert!(a_sp > a_lo && a_sp <= a_hi, "task a SP not on its stack");
    assert!(b_sp > b_lo && b_sp <= b_hi, "task b SP not on its stack");
    assert_ne!(a_sp, b_sp);
    let idle = current_sp();
    assert!(idle > idle_lo && idle <= idle_hi, "yield returned off idle");
}

#[cfg(test)]
#[test_case]
fn yield_round_robin_resumes_both() {
    static A_HITS: AtomicU64 = AtomicU64::new(0);
    static B_HITS: AtomicU64 = AtomicU64::new(0);

    fn ping_a() {
        A_HITS.fetch_add(1, Ordering::SeqCst);
        yield_now();
        A_HITS.fetch_add(1, Ordering::SeqCst);
    }
    fn ping_b() {
        B_HITS.fetch_add(1, Ordering::SeqCst);
        yield_now();
        B_HITS.fetch_add(1, Ordering::SeqCst);
    }

    reset_workers();
    A_HITS.store(0, Ordering::SeqCst);
    B_HITS.store(0, Ordering::SeqCst);
    assert!(spawn(ping_a).is_some());
    assert!(spawn(ping_b).is_some());
    let before = SWITCHES.load(Ordering::SeqCst);
    for _ in 0..YIELD_BOUND {
        if A_HITS.load(Ordering::SeqCst) >= 2 && B_HITS.load(Ordering::SeqCst) >= 2 {
            break;
        }
        yield_now();
    }
    assert!(A_HITS.load(Ordering::SeqCst) >= 2);
    assert!(B_HITS.load(Ordering::SeqCst) >= 2);
    assert!(SWITCHES.load(Ordering::SeqCst) > before);
}
