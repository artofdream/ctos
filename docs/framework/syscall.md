# SVC syscall ABI (Track A / A1 / ADR-021)

**ABI + CRT miles.** App hosting (loader, VFS, OS/app slots) stays **Planned**. Track A is still incomplete after A2. Not Linux. Not POSIX.

Kernel contract: [ADR-021](../03-adr/ADR-021-svc-syscall-abi.md). CRT / `libctos`: [ADR-022](../03-adr/ADR-022-libctos-crt.md). Parent plan: [issue #31](https://github.com/artofdream/ctos/issues/31). A1: [issue #32](https://github.com/artofdream/ctos/issues/32). A2: [issue #33](https://github.com/artofdream/ctos/issues/33). Code: `src/syscall.rs`, `libctos/`, `user/hello-libctos/`.

## Calling convention

AArch64 `SVC #<n>` where `n` is the syscall number. Arguments in `x0`, `x1`, `x2`. Return in `x0`.

Reserved **0–2** are ADR-013 probes (`#0` first-mile return, `#1` standing, `#2` restore). `libctos` must not issue them. There is no hosted app yet.

## Public numbers

| Number | Name | Args | Effect |
| --- | --- | --- | --- |
| 16 | `exit` | `x0` = status | Halt the EL0 trip; return to the EL1 caller. Status is recorded. Not a process table. |
| 17 | `uart_write` | `x0` = pointer, `x1` = length | Write up to 64 user-mapped **and** kernel-mapped bytes to the virt PL011. `\n` → `\r\n`. Reject (including non-canonical / TTBR1 aliases) → `x0 = 0`. |
| 18 | `yield` | none | Dispatch-only hint; return to EL0. Does not switch EL1 tasks ([ADR-010](../03-adr/ADR-010-cooperative-rr-el1.md)). |

Unknown `SVC` immediates park (fail-closed).

## Probe

Serial `svc: yield` / `svc: user-hi` / `svc: uart` / `svc: exit` / `svc: ok`. `#[test_case]` `el0_svc_abi_yield_uart_exit` + `el0_uart_write_rejects_kernel_data` + `el0_uart_write_rejects_high_alias`. File presence is not that probe.

## `libctos` (A2)

A `no_std` crate wraps the three public numbers. A hello payload linked against it is copied onto the standing EL0 page (not a guest ELF loader). Serial `libctos: hi` / `libctos: ok` / `libctos: linked`. File presence is not that probe.

## Still Planned (Track A)

ELF loader, standing EL0 as normal mode, isolation completion, VFS / memfs, virtio-blk, sample apps, OS/app slots (A3–A9 on #31).
