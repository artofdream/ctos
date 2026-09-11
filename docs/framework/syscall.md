# SVC syscall ABI (Track A / A1 / ADR-021)

**ABI mile: documented here.** App hosting (loader, `libctos`, VFS, OS/app slots) stays **Planned**. Not Linux. Not POSIX.

Contract: [ADR-021](../03-adr/ADR-021-svc-syscall-abi.md). Parent plan: [issue #31](https://github.com/artofdream/ctos/issues/31). This mile: [issue #32](https://github.com/artofdream/ctos/issues/32). Code: `src/syscall.rs`.

## Calling convention

AArch64 `SVC #<n>` where `n` is the syscall number. Arguments in `x0`, `x1`, `x2`. Return in `x0`.

Reserved **0–2** are ADR-013 probes (`#0` first-mile return, `#1` standing, `#2` restore). A freestanding app must not issue them.

## Public numbers

| Number | Name | Args | Effect |
| --- | --- | --- | --- |
| 16 | `exit` | `x0` = status | Halt the EL0 trip; return to the EL1 caller. Status is recorded. Not a process table. |
| 17 | `uart_write` | `x0` = pointer, `x1` = length | Write up to 64 user-mapped **and** kernel-mapped bytes to the virt PL011. `\n` → `\r\n`. Reject → `x0 = 0`. |
| 18 | `yield` | none | Dispatch-only hint; return to EL0. Does not switch EL1 tasks ([ADR-010](../03-adr/ADR-010-cooperative-rr-el1.md)). |

Unknown `SVC` immediates park (fail-closed).

## Probe

Serial `svc: yield` / `svc: user-hi` / `svc: uart` / `svc: exit` / `svc: ok`. `#[test_case]` `el0_svc_abi_yield_uart_exit` + `el0_uart_write_rejects_kernel_data`. File presence is not that probe.

## Still Planned (Track A)

ELF loader, `libctos` crate, standing EL0 as normal mode, isolation completion, VFS / memfs, virtio-blk, sample apps (A2–A9 on #31).
