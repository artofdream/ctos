# In-tree sample recipes (Track A / A8)

These are **rebuild recipes**, not applications you drop onto a running OS. Each class matches a Verified serial marker. Status lives in the [honesty ledger](../docs/framework/honesty-ledger.md). Visitor hub: [What can run today](../docs/overview/what-can-run.md).

There is no `examples/` crate and no second QEMU `-kernel`. One `cargo build` of the kernel (plus `build.rs` for the hello) is the rebuild.

| Class | Tree | Rebuild | Probe (do not invent extras) |
| --- | --- | --- | --- |
| Cooperative UART workers (EL1) | `src/sched.rs` (`task_a` / `task_b`) | `cargo build` then `./scripts/qemu-smoke.sh` | `sched: task a` / `sched: task b` / `sched: ok` |
| UART RX echo gadget | `src/uart.rs` + `scripts/qemu-serial-inject.py` | same | `input: rx 0x41` |
| Standing EL0 / `libctos` hello | [`hello-libctos/`](hello-libctos/README.md), `libctos/`, `build.rs`, `src/loader.rs` | same (or build the user crate first; A2–A4 load FAT `/hello`) | `libctos: hi` / `libctos: ok` / `loader: ok` / `el0: task-ok` |
| OS/app slot first cut (A9) | `target/hello-libctos.elf` on FAT `/hello` (`scripts/mkfat16.py --app`) | same (`qemu-smoke` publishes + attaches) | `slot: fat` / `slot: mapped` / `slot: ok` |
| memfs named buffers (optional) | `src/vfs.rs`; EL0 trampoline `/eprobe` in `src/syscall.rs` | same | `fs: create` / `fs: write` / `fs: read` / `fs: el0` / `fs: ok` |
| FAT16 `/probe` (optional) | `scripts/mkfat16.py`, `src/virtio.rs`, `src/fat.rs` | same (`qemu-smoke` attaches `-drive`) | `blk: ok` / `fat: ok` (`/probe` = `fat-hi`) |

Not claimed: POSIX, Linux ports, product app hosting, “secure OS.”

A `libctos` hello that calls `fs_open("/probe")` is **not** in this tree. Wrappers exist; the hello does not use them. The Verified EL0 VFS trip is the `/eprobe` trampoline.
