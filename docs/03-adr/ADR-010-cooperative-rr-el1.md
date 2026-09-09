# ADR-010 — Cooperative round-robin on EL1

- Status: Accepted
- Date: 2026-09-09

## Context

[FR-11](../02-requirements/fr-nfr.md) requires cooperative or simple round-robin task switching. Roadmap M9 is that path. The heap already exists ([ADR-009](ADR-009-first-fit-heap.md)); paging is an identity map ([ADR-008](ADR-008-identity-map-frame-allocator.md)). The kernel runs at EL1 with `SPSel = 0` so `SP` is `SP_EL0` ([ADR-005](ADR-005-fatal-exception-stack.md)).

Options:

1. **Async/await executor.** Extra `Future` infrastructure for two markers.
2. **Timer preemption.** Would steal the M5 CNTP path and needs IRQ-safe yield. Not required to prove FR-11.
3. **Cooperative yield** that saves AAPCS64 callee-saved GPRs on a per-task stack and switches `SP`.
4. **SMP / EL0 processes.** Out of scope (vision Out list).

The callee-saved layout is not obvious next to the exception frame in [ADR-004](ADR-004-el1-vbar-brk.md) (that frame is `x0`–`x30` + `ELR`/`SPSR`/`ESR` on `SP_EL1`). A yield must not touch the exception stacks.

## Decision

1. **Cooperative only.** Tasks call `sched::yield_now()`. No timer slice. DAIF.I stays masked during the hello/test probe (M5 still remasks after its own tick).
2. **AAPCS64 callee-saved switch** in `context_switch`: save/restore `x19`–`x28`, `x29`, `x30` on the task stack; store SP in the task slot; load the next SP. No SIMD/FP ([ADR-003](ADR-003-primary-isa-aarch64.md) soft-float). Not the exception context.
3. **Idle slot 0** is `kernel_main` / the test runner on the linker thread stack. **Two workers** get 8 KiB stacks from the M8 heap (`Vec<u8>`, not a stack-allocated `[u8; N]` that would overflow `SP_EL0` during `Box::new`). Identity VA == PA.
4. **Round-robin among `Ready` slots.** A worker that returns is `Done` (trampoline). Idle stays `Ready` so control returns to the caller of `yield_now`. Unlock the scheduler mutex before `context_switch`.
5. **Serial markers** `sched: task a`, `sched: task b`, then `sched: ok` after both SPs land on distinct heap stacks. Fail closed on `sched: probe missed`.
6. **Not preemptive, not SMP, not EL0, not async.** A later preemptive or process ADR is a new file.

## Consequences

- `scripts/qemu-smoke.sh` requires the three `sched:` strings after `heap: ok`, then still requires M2–M8 strings.
- Two 8 KiB worker stacks come out of the 64 KiB heap. M8 `Box`/`Vec` probes drop before spawn.
- Exception / fatal stacks are unchanged. A yield never runs from an IRQ handler.
- This ADR does not claim Raspberry Pi, preemption, or userspace.
