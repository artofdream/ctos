# What can run today

Do not say “applications run on ctos.” These are the **probed guest examples** on QEMU `virt`. Each needs a matching [honesty ledger](honesty-ledger.md) row. Isolation, POSIX, and a userspace ABI stay **Planned**.

Hub: [overview.md](overview.md). How to add something: [building-or-porting.md](building-or-porting.md). Frozen IDs: [FR-08](../02-requirements/fr-nfr.md) (UART RX), [FR-11](../02-requirements/fr-nfr.md) (coop yield).

Site source of truth: [What can run today](../overview/what-can-run.md) (A8 rebuild recipes). In-tree index: `user/README.md`. This page is extra stance (walkthroughs). HTTPS at https://ctos.artof.link is **Verified** (2026-09-11 after #30).

**One rebuild for every sample:** `cargo build` then `./scripts/qemu-smoke.sh`. There is no separate app runner. A8 documents that path. A9 first cut adds `slot:*` when FAT `/hello` loads. Cross-update stays **Planned**.

## Sample: cooperative EL1 UART workers (M9)

**What it is.** Two heap-backed EL1 workers take turns. Each prints one UART line, then the idle thread sees both flags and prints `sched: ok`. Cooperative only — a task runs until it returns or calls `yield_now()`. Not a timer slice. Not SMP. Not EL0.

**Where.** `src/sched.rs` (`task_a` / `task_b` / `run_two_tasks`). Decision: [ADR-010](../03-adr/ADR-010-cooperative-rr-el1.md).

**Rebuild.** Edit those functions. `cargo build` then `./scripts/qemu-smoke.sh`. Recipe hub: [what-can-run.md](../overview/what-can-run.md).

**How it works.**

1. Heap must already be up (`src/heap.rs`). Worker stacks are 8 KiB `Vec<u8>` boxes, not the linker `SP_EL0` stack.
2. `spawn(task_a)` and `spawn(task_b)` install AAPCS64 callee-saved frames. After ADR-020 the first `ret` lands on the high-VA alias of the trampoline.
3. Idle (`kernel_main` / the test runner) calls `yield_now()` until both atomics are set, or a bound is hit.
4. Each worker stores its SP, writes a raw UART line, sets its flag, and returns (`Done`).

**Probe (do not invent extra markers).**

| Serial | Meaning |
| --- | --- |
| `sched: task a` | Worker A ran on a heap stack |
| `sched: task b` | Worker B ran on a heap stack |
| `sched: ok` | Both ran; smoke greps all three |

`#[test_case]` `two_tasks_run_on_distinct_heap_stacks` and `yield_round_robin_resumes_both` close the same claim. Idle tip Docker on `main` `e80dc93`: those strings still print on the hello path (50 tests). File presence is not that probe.

### Heartbeat / counter variant (not in tree)

Same shape as `task_a` / `task_b`, not a new subsystem. A worker that loops, increments an `AtomicU64`, prints `sched: beat n=<n>`, and yields would be a **counter / heartbeat** gadget.

That variant is **not** in `src/sched.rs` today. There is no `sched: beat` smoke string. Adding it is the easiest in-tree exercise ([building-or-porting.md](building-or-porting.md)): one extra `fn`, one `spawn`, one milestone → one PR. Until a probe greps the new marker, treat it as **Planned**, not Verified.

## Sample: UART RX echo gadget (M6)

**What it is.** The guest polls the virt PL011 RX FIFO and prints the first byte the host injects. On the smoke path that byte is `0x41` (`'A'`). That is one-shot observe, not a line editor and not a shell.

**Where.** `src/uart.rs` (`observe_rx` / `observe_probe_byte`). Host inject: `scripts/qemu-serial-inject.py` (default `CTOS_INPUT_BYTE=0x41`). Decision: [ADR-007](../03-adr/ADR-007-pl011-uart-rx.md).

**Rebuild.** Same `cargo build` + `./scripts/qemu-smoke.sh` (the script injects `0x41`). Recipe: [what-can-run.md](../overview/what-can-run.md).

**How it works.**

1. After the hello path has printed earlier markers, `observe_probe_byte` spins on `UARTFR.RXFE` until a byte arrives or `CNTPCT` times out.
2. IRQs stay masked on this path. There is no UART IRQ in M6. Do not `wfi` waiting for RX.
3. The expected byte prints `input: rx 0x41`. Any other byte prints `input: rx 0xNN (unexpected)` and fails the probe. Timeout prints `input: rx missed`.

**Probe.**

| Serial | Meaning |
| --- | --- |
| `input: rx 0x41` | Host-injected `0x41` landed in the FIFO |
| `input: rx missed` | Fail-closed (smoke rejects this) |

`#[test_case]` `uart_rx_fifo_empty_without_host_byte` only proves the FIFO is empty under `cargo test` (no inject). Character proof is the hello serial row.

This is **not** interactive echo, virtio-keyboard, or a TTY. A later line-oriented gadget would be a new milestone with its own marker.

## Sample: standing EL0 stub (SVC enter / leave)

**What it is.** A bounded user context on the user TTBR0. The kernel `ERET`s to EL0, the payload announces with `SVC #1`, runs one user `MOVZ`, then `SVC #2` restores EL1. `el0::is_active()` is true only for that lifetime.

**Where.** `src/el0.rs` (standing install) and the lower-EL sync path in `src/exception.rs`. Direction: [ADR-013](../03-adr/ADR-013-el0-isolation-direction.md). Narrative: [el0.md](el0.md).

**How it works.**

1. First miles already proved enter/return (`el0: svc`), UXN fetch (`el0: nx kernel`), and no kernel `.data` read (`el0: no kernel read`).
2. Standing payload on the EL0 window: `SVC #1` → serial `el0: standing` (stay at EL0) → `MOVZ X1, #0x51A4` → `SVC #2` → serial `el0: restored` (back to EL1).
3. Saved user PC / SP / TTBR0 live only while `is_active()` is true. The handler clears that flag on restore.

**Probe.**

| Serial | Meaning |
| --- | --- |
| `el0: standing` | `SVC #1` taken; still at EL0 |
| `el0: restored` | `SVC #2` taken; back to EL1 |
| `el0: ok` | First-mile bundle including standing |

`#[test_case]` `standing_el0_enter_leave` closes enter/leave. After A1 the standing stub also runs the documented ABI trip (`svc: yield` / `svc: user-hi` / `svc: uart` / `svc: exit` / `svc: ok`) — [syscall.md](syscall.md). After A2 a hello **linked against `libctos`** prints `libctos: hi` / `libctos: ok` ([ADR-022](../03-adr/ADR-022-libctos-crt.md)). After A3 the same ELF is guest-parsed (`loader: ok`, [ADR-023](../03-adr/ADR-023-elf-pt-load-loader.md)). After A4 that loaded image is a standing **task** until `exit` (`el0: task-ok`, [ADR-024](../03-adr/ADR-024-standing-el0-normal.md)); an unexpected fault restores fail-closed (`el0: restore-fail`). Lower-EL IRQ still parks. PAN on `-cpu cortex-a57` is **Planned**. This is **not** a user process, not POSIX, and not “EL0 isolated.” The ABI + CRT + loader + standing-task miles are not app hosting.

**Rebuild the hello.** Source is `user/hello-libctos/` (`user/hello-libctos/README.md`). A kernel `cargo build` publishes it via `build.rs` onto FAT `/hello`. Standalone:

```bash
cargo build --release \
  --manifest-path user/hello-libctos/Cargo.toml \
  --target user/hello-libctos/aarch64-ctos-user.json
```

That ELF still has to land on FAT `/hello` to run. The hello does **not** call `fs_open`. The Verified EL0 VFS trip is `/eprobe` (`fs: el0`).

## Sample: memfs named-buffer probe (A6)

**What it is.** Thin VFS + in-RAM buffers. Kernel creates `/kprobe`, writes, reads. An EL0 trampoline does the same for `/eprobe` (`memfs-el0`). Not POSIX. Not a directory tree.

**Where.** `src/vfs.rs`, `src/syscall.rs` (`el0_fs_svc_roundtrip`). Decision: [ADR-027](../03-adr/ADR-027-thin-vfs-memfs.md).

**Rebuild.** Same `cargo build` + `./scripts/qemu-smoke.sh`.

**Probe.** `fs: create` / `fs: write` / `fs: read` / `fs: el0` / `fs: ok`. File presence is not that probe.

## Sample: FAT16 `/probe` on virtio-blk (A7)

**What it is.** Guest programs virtio-mmio block and `vfs::open("/probe")` reads `fat-hi` from a host-built FAT16 image. Same `open` as memfs. FAT is **read-only**.

**Where.** `src/virtio.rs`, `src/fat.rs`, `scripts/mkfat16.py`. Decision: [ADR-028](../03-adr/ADR-028-virtio-blk-fat16.md).

**Rebuild.** Same smoke. The script writes `target/fat16.img` and attaches `-drive if=none,file=…,id=hd0 -device virtio-blk-device,drive=hd0`. Host `-drive` without `fat: ok` is not the probe.

**Probe.** `blk: virtio` / `blk: cap` / `blk: rw` / `blk: ok` and `fat: mount` / `fat: read` / `fat: ok`. Do not say “supports FAT.”

## Also on the same hello path

These are kernel sensors, not apps. Cite the ledger SHA if you quote a number. Recent A7 smoke still printed:

- UART `Hello World!`
- `paging: ok`, `heap: ok` (`Box` / `Vec` on the first-fit heap)
- `timer: tick` (GICv2 + CNTP)
- `wx: ok`, `guard: ok`, `ro: ok`
- `ident: reloc` / `ident: live` / `ident: rodata` / `ident: ok`
- `svc: ok` / `libctos: ok` / `loader: ok` / `el0: task-ok`
- `fs: ok` / `blk: ok` / `fat: ok`
- `exception: sync BRK` then `exception: fatal nested`

## What cannot run today

Explicit **no**. Do not paper over these with a “porting guide.”

| Want | Why not |
| --- | --- |
| A Linux ELF / glibc / musl binary | No `exec`, no ELF loader, no syscall table. Custom target is `os: none`. |
| A shell (`sh`, bash) or line-oriented TTY | RX probe is one injected byte. No line discipline. |
| Python, Node, or any hosted interpreter | Needs a process ABI, heap policy, and usually a filesystem. |
| Network / sockets / HTTP | No virtio-net, no stack, no sockets. |
| POSIX / Linux disk apps | memfs (A6) + one read-only FAT16 file on virtio-blk (A7 / [ADR-028](../03-adr/ADR-028-virtio-blk-fat16.md)). Not POSIX. Not FAT32. Stance: [filesystem.md](filesystem.md). |
| SMP / a second CPU / preemptive threads | M9 is cooperative EL1 on one vCPU. |
| Isolated userspace / “an app you compile and exec” | Standing EL0 is a stub. PAN **enable** + remaining identity RAM / `_start` + umbrella isolation stay **Planned**. Identity `.rodata`/`.data`/heap are torn (ADR-025/037/038). Gaps: [host-apps.md](host-apps.md). |
| OCI / Docker / k8s **in the guest** | **No — non-goal** ([ADR-029](../03-adr/ADR-029-containers-nongoal.md)). Host `docker-smoke.sh` only builds the kernel. |
| Raspberry Pi or any board other than QEMU `virt` | Unprobed. Do not copy virt Verified onto hardware. |
| GPU / desktop / windowing / virtio devices | Out of scope on this horizon. |

Isolation, PAN **enable**, and tearing identity `.data` / heap stay **Planned**. Identity `.rodata` is a probed A5 mile. See [el0.md](el0.md).
