# Session memory — 2026-09-09 (M9)

Cooperative yield only: save x19–x30 on the task stack, switch SP_EL0. Do not reuse the exception frame. Worker stacks are heap `Vec<u8>` — do not `Box::new([0u8; 8192])` (that constructs on the thread stack). Unlock the sched mutex before `context_switch`. Idle slot 0 must stay Ready so `yield_now` returns to kernel_main / tests. Do not start preemption. Do not claim Raspberry Pi.
