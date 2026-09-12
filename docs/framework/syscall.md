# SVC syscall ABI (Track A / A1 / ADR-021)

**ABI + CRT + loader + standing-task miles.** App hosting (VFS, OS/app slots) stays **Planned**. Track A is still incomplete after A4. Not Linux. Not POSIX.

Kernel contract: [ADR-021](../03-adr/ADR-021-svc-syscall-abi.md). CRT / `libctos`: [ADR-022](../03-adr/ADR-022-libctos-crt.md). Guest loader: [ADR-023](../03-adr/ADR-023-elf-pt-load-loader.md). Standing-as-normal: [ADR-024](../03-adr/ADR-024-standing-el0-normal.md). Parent plan: [issue #31](https://github.com/artofdream/ctos/issues/31). A1: [issue #32](https://github.com/artofdream/ctos/issues/32). A2: [issue #33](https://github.com/artofdream/ctos/issues/33). A3: [issue #34](https://github.com/artofdream/ctos/issues/34). A4: [issue #35](https://github.com/artofdream/ctos/issues/35). Code: `src/syscall.rs`, `libctos/`, `user/hello-libctos/`, `src/loader.rs`, `src/el0.rs`.

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

A `no_std` crate wraps the three public numbers. A hello payload linked against it is copied onto the standing EL0 page (A2) **and** parsed as ELF64 `PT_LOAD` on the guest (A3). Serial `libctos: hi` / `libctos: ok` / `libctos: linked`. File presence is not that probe.

## Guest loader (A3)

The kernel parses the embedded hello ELF, maps each `PT_LOAD` into the user map-window, and `ERET`s to `e_entry` ([ADR-023](../03-adr/ADR-023-elf-pt-load-loader.md)). Serial `loader: mapped` / `loader: ok`. Not a Linux ELF ABI. Not `PT_INTERP`. The image is still bundled (no VFS). File presence is not that probe.

## Standing task (A4)

The A3 loader is how a payload appears. A4 makes standing EL0 the **supported path**: `is_active()` is true for the loaded task until `exit`; an unexpected fault restores fail-closed ([ADR-024](../03-adr/ADR-024-standing-el0-normal.md)). Serial `el0: task-enter` / `el0: task-active` / `el0: task-exit` / `el0: task-restored` / `el0: restore-fail` / `el0: task-ok`. Not isolation. Not a process table. File presence is not that probe.

## Still Planned (Track A)

Isolation completion, VFS / memfs, virtio-blk, sample apps, OS/app slots (A5–A9 on #31).
