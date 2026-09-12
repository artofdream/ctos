# ADR-024 — Standing EL0 as normal mode

- Status: Accepted (standing-task mile Verified when the serial / tests pass; Track A / app hosting still Planned)
- Date: 2026-09-12

## Context

Track A ([issue #31](https://github.com/artofdream/ctos/issues/31)) wants a loaded freestanding image to **run** at EL0, not only enter and leave. [Issue #35](https://github.com/artofdream/ctos/issues/35) is this mile.

[ADR-013](ADR-013-el0-isolation-direction.md) already has a bounded standing context: `SVC #1` stays at EL0, a user `MOVZ` runs, `SVC #2` restores EL1. `is_active()` is true only for that lifetime. That is a **smoke probe**, not the supported path for a loaded app.

[ADR-021](ADR-021-svc-syscall-abi.md) / [ADR-022](ADR-022-libctos-crt.md) / [ADR-023](ADR-023-elf-pt-load-loader.md) already `ERET` to a payload and come back on `SYS_EXIT`. They reuse `install_standing` as a flag. A leftover `is_active()` after `ERET` fails the probe. That is still a one-shot trip, not “standing EL0 is how a loaded image runs.”

A4 keeps the A3 loader as the way a payload appears. It does **not** add VFS, slots, PAN, or a Linux `exec`. Existing `svc:*` / `libctos:*` / `loader:*` / `el0: standing` / `el0: restored` markers stay.

## Hypotheses

| ID | Option | Notes |
| --- | --- | --- |
| **H1 (chosen)** | Distinguish **probe** vs **task** standing. A task is the supported path: load (A3) → `is_active()` true → stay at EL0 for `yield` / `uart_write` → `exit` restores. Unexpected lower-EL sync while a task is standing restores fail-closed instead of parking. | Proves real-task `is_active` and a restore that cannot leave a dangling standing context. |
| **H2 (rejected)** | Treat the ADR-013 dual-SVC as “normal mode.” | Still a one-shot `SVC #1` / `#2` trampoline. Does not run a loaded app until `exit`. |
| **H3 (rejected)** | Claim isolation / PAN / process table while promoting standing. | Inflates the mile. Isolation is A5 ([issue #36](https://github.com/artofdream/ctos/issues/36)). |

## Decision

1. **Two standing kinds.** `install_standing` remains the ADR-013 / A1–A3 **probe** (dual-SVC or ABI trampoline). `install_task` is the A4 **task**. `is_active()` is true for both. `is_task()` is true only for a task. Saved user PC / SP / TTBR0 live only while `is_active()`.
2. **Supported path.** The A3 guest ELF loader maps `PT_LOAD` from FAT `/hello` ([ADR-031](ADR-031-track-a-leftovers.md)), then `install_task(e_entry, stack_top, user_ttbr0)` and `ERET`. The loaded image runs at EL0 until `SYS_EXIT`. `SYS_YIELD` / `SYS_UART_WRITE` stay at EL0 and must observe `is_active()` (`el0: task-active`). A2’s memcpy path stays so `libctos:*` markers remain (same FAT file; no embed).
3. **Fail-closed restore.** `SYS_EXIT` on a task prints `el0: task-exit` / `el0: task-restored` and clears `is_active()`. An unexpected lower-EL sync (unhandled SVC, translation/permission abort, and anything that would otherwise park) **while a task is standing** prints `el0: restore-fail`, clears `is_active()`, and returns to the EL1 caller. It does **not** park. Probe standing still parks on unexpected sync (A1–A3 unchanged). After `ERET` returns, leftover `is_active()` is Failed (caller force-clears and fails the probe).
4. **Fail-closed probe.** Hello serial `el0: task-enter` + `el0: task-active` + `el0: task-exit` + `el0: task-restored` + `el0: restore-fail` + `el0: task-ok`. `scripts/qemu-smoke.sh` greps those and rejects `el0: task missed`. `#[test_case]` covers the loaded-hello task trip and the fault-restore trip. Existing `svc:` / `libctos:` / `loader:` / `el0: standing` / `el0: restored` markers stay.
5. **Honesty.** Say “a loaded freestanding image stood at EL0 until `exit`, and an unexpected fault restored fail-closed” only when the serial / tests pass. Do **not** say: app hosting is done, POSIX, userspace, “EL0 isolated,” Linux `exec`, or “secure OS.” Track A A5–A9 stay Planned.
6. **NFR-10 text** is revised in place (ID unchanged) to name this standing-task mile. Do not mint FR-16+ or NFR-15+.

## Consequences

- Code: `src/el0.rs` (`install_task`, `restore_exit`, `restore_fault`), `src/loader.rs` (`run_hello_as_task`), `src/exception.rs` (task restore instead of park), `src/syscall.rs` (`note_active_while_standing`), `scripts/qemu-smoke.sh`.
- Docs: [el0.md](../framework/el0.md), [syscall.md](../framework/syscall.md), [track-a.md](../04-roadmap/track-a.md). Roadmap cites #35 / Track A #31.
- A5 (isolation completion — remaining identity `.rodata` / `.data` / heap tear; PAN only if the probe CPU implements it **and** an ADR says so) is not this PR.
