# SVC syscall ABI (Track A / A1 / ADR-021)

**ABI + CRT + loader + standing-task + memfs + FAT16 + slot first-cut miles.** Sample **rebuild recipes** are A8 (docs). A9 is two artifacts + FAT `/hello` ([ADR-030](../03-adr/ADR-030-os-app-slots.md)). Cross-update and “app hosting is done” stay **Planned**. Not Linux. Not POSIX.

Kernel contract: [ADR-021](../03-adr/ADR-021-svc-syscall-abi.md). CRT / `libctos`: [ADR-022](../03-adr/ADR-022-libctos-crt.md). Guest loader: [ADR-023](../03-adr/ADR-023-elf-pt-load-loader.md). Standing-as-normal: [ADR-024](../03-adr/ADR-024-standing-el0-normal.md). Thin VFS + memfs: [ADR-027](../03-adr/ADR-027-thin-vfs-memfs.md). virtio-blk + FAT16: [ADR-028](../03-adr/ADR-028-virtio-blk-fat16.md). Parent plan: [issue #31](https://github.com/artofdream/ctos/issues/31). A1: [issue #32](https://github.com/artofdream/ctos/issues/32). A2: [issue #33](https://github.com/artofdream/ctos/issues/33). A3: [issue #34](https://github.com/artofdream/ctos/issues/34). A4: [issue #35](https://github.com/artofdream/ctos/issues/35). A6: [issue #37](https://github.com/artofdream/ctos/issues/37). Code: `src/syscall.rs`, `src/vfs.rs`, `libctos/`, `user/hello-libctos/`, `src/loader.rs`, `src/el0.rs`.

## Calling convention

AArch64 `SVC #<n>` where `n` is the syscall number. Arguments in `x0`, `x1`, `x2`. Return in `x0`.

Reserved **0–2** are ADR-013 probes (`#0` first-mile return, `#1` standing, `#2` restore). `libctos` must not issue them. There is no hosted app yet.

## Public numbers

| Number | Name | Args | Effect |
| --- | --- | --- | --- |
| 16 | `exit` | `x0` = status | Halt the EL0 trip; return to the EL1 caller. Status is recorded. Not a process table. |
| 17 | `uart_write` | `x0` = pointer, `x1` = length | Write up to 64 user-mapped **and** kernel-mapped bytes to the virt PL011. `\n` → `\r\n`. Reject (including non-canonical / TTBR1 aliases) → `x0 = 0`. |
| 18 | `yield` | none | Dispatch-only hint; return to EL0. Does not switch EL1 tasks ([ADR-010](../03-adr/ADR-010-cooperative-rr-el1.md)). |
| 19 | `fs_create` | `x0` = path, `x1` = length | Create an empty memfs name and a handle. Path `/` + `[a-z0-9_-]`. | fd `>= 1`, or `0`. |
| 20 | `fs_open` | `x0` = path, `x1` = length | Open an existing memfs name. | fd `>= 1`, or `0`. |
| 21 | `fs_read` | `x0` = fd, `x1` = buf, `x2` = length | Copy from the handle offset into a user-mapped **and** kernel-mapped range (cap 64). | bytes, or `u64::MAX` if rejected. `0` is a valid empty read. |
| 22 | `fs_write` | `x0` = fd, `x1` = buf, `x2` = length | Copy into the file at the handle offset (cap 64, file cap 256). | bytes, or `0`. |
| 23 | `fs_close` | `x0` = fd | Drop the handle. The file stays. | `0` on success, `u64::MAX` if rejected. |

Unknown `SVC` immediates park (fail-closed). Not POSIX. Not Linux VFS.

Track B B2 maps these numbers against Linux AArch64 (`svc #0`, `x8`): [linux-aarch64-syscall-gap.md](../research/linux-aarch64-syscall-gap.md). That page is inspection only. **Not claiming Linux userspace.** Do not retarget 16–23 to Linux `x8`.

## Probe

Serial `svc: yield` / `svc: user-hi` / `svc: uart` / `svc: exit` / `svc: ok`. `#[test_case]` `el0_svc_abi_yield_uart_exit` + `el0_uart_write_rejects_kernel_data` + `el0_uart_write_rejects_high_alias`. File presence is not that probe.

## `libctos` (A2)

A `no_std` crate wraps the three public numbers. A hello payload linked against it is copied onto the standing EL0 page (A2) **and** parsed as ELF64 `PT_LOAD` on the guest (A3). Serial `libctos: hi` / `libctos: ok` / `libctos: linked`. File presence is not that probe.

## Guest loader (A3)

The kernel parses the embedded hello ELF, maps each `PT_LOAD` into the user map-window, and `ERET`s to `e_entry` ([ADR-023](../03-adr/ADR-023-elf-pt-load-loader.md)). Serial `loader: mapped` / `loader: ok`. Not a Linux ELF ABI. Not `PT_INTERP`. A2–A4 still bundle the image. File presence is not that probe.

## Standing task (A4)

The A3 loader is how a payload appears. A4 makes standing EL0 the **supported path**: `is_active()` is true for the loaded task until `exit`; an unexpected fault restores fail-closed ([ADR-024](../03-adr/ADR-024-standing-el0-normal.md)). Serial `el0: task-enter` / `el0: task-active` / `el0: task-exit` / `el0: task-restored` / `el0: restore-fail` / `el0: task-ok`. Not isolation. Not a process table. File presence is not that probe.

## memfs (A6)

In-RAM named buffers behind a thin VFS ([ADR-027](../03-adr/ADR-027-thin-vfs-memfs.md), [filesystem.md](filesystem.md)). Serial `fs: create` / `fs: write` / `fs: read` / `fs: el0` / `fs: ok`. File presence is not that probe.

## FAT16 (A7)

Same VFS `open` on virtio-blk ([ADR-028](../03-adr/ADR-028-virtio-blk-fat16.md)). Serial `blk: ok` / `fat: ok`. `/probe` is the known FAT16 file. Write on a FAT handle is `ReadOnly`. File presence is not that probe.

## Sample recipes (A8)

Rebuild recipes for the probed classes: [what-can-run.md](../overview/what-can-run.md), in-tree `user/README.md`. Coop UART, RX echo, standing EL0 / `libctos` hello; optional memfs + FAT16. No new syscall numbers. File presence is not a new runtime.

## OS/app slots (A9)

Host kernel ELF + published `hello-libctos.elf`. Guest reads FAT `/hello` through the same VFS and maps it with the A3 loader ([ADR-030](../03-adr/ADR-030-os-app-slots.md)). Serial `slot: fat` / `slot: mapped` / `slot: ok` / `perf: app-load`. No embed fallback on this path. A2–A4 still embed. Cross-update stays Planned. File presence is not that probe.

## Still Planned (Track A)

A9 cross-update (same app on OS n and n+1). Isolation **enable** (PAN) and identity `.data` / heap tear stay Planned.
