# In-tree sample recipes (Track A / A8 + ADR-059 + ADR-061 + ADR-062 + ADR-068 + ADR-071 + ADR-075)

These are **rebuild recipes**, not applications you drop onto a running OS. Each class matches a Verified serial marker. Status lives in the [honesty ledger](../docs/framework/honesty-ledger.md). Visitor hub: [What can run today](../docs/overview/what-can-run.md).

There is no `examples/` crate and no second QEMU `-kernel`. One `cargo build` of the kernel (plus `build.rs` for the freestanding samples) is the rebuild.

| Class | Tree | Rebuild | Probe (do not invent extras) |
| --- | --- | --- | --- |
| Cooperative UART workers (EL1) | `src/sched.rs` (`task_a` / `task_b`) | `cargo build` then `./scripts/qemu-smoke.sh` | `sched: task a` / `sched: task b` / `sched: ok` |
| UART RX echo gadget | `src/uart.rs` + `scripts/qemu-serial-inject.py` | same | `input: rx 0x41` |
| Standing EL0 / `libctos` hello | [`hello-libctos/`](hello-libctos/README.md), `libctos/`, `build.rs`, `src/loader.rs` | same (or build the user crate first; A2–A4 load FAT `/hello`) | `libctos: hi` / `libctos: ok` / `loader: ok` / `el0: task-ok` |
| Standing EL0 / `libctos` VFS sample | [`fs-libctos/`](fs-libctos/README.md), `libctos/`, `build.rs`, `src/fsdemo.rs` | same (FAT `/fsdemo`) | `libctos: fs-hi` / `libctos: fs-ok` / `fsdemo: ok` |
| Standing EL0 / `libctos` FAT sample | [`fat-libctos/`](fat-libctos/README.md), `libctos/`, `build.rs`, `src/fatdemo.rs` | same (FAT `/fatdemo`) | `libctos: fat-hi` / `libctos: fat-ok` / `fatdemo: ok` |
| Standing EL0 / `libctos` yield sample | [`yield-libctos/`](yield-libctos/README.md), `libctos/`, `build.rs`, `src/yldemo.rs` | same (FAT `/yldemo`) | `libctos: yld-hi` / `libctos: beat` / `libctos: yld-ok` / `yldemo: ok` |
| Standing EL0 / `libctos` net sample | [`net-libctos/`](net-libctos/README.md), `libctos/`, `build.rs`, `src/netdemo.rs` | same (FAT `/netdemo`) | `libctos: net-hi` / `libctos: net-mac` / `libctos: net-ok` / `netdemo: ok` |
| Standing EL0 / `libctos` UDP DNS sample | [`udp-libctos/`](udp-libctos/README.md), `libctos/`, `build.rs`, `src/udpdemo.rs` | same (FAT `/udpdemo`) | `libctos: udp-hi` / `libctos: udp-ok` / `udpdemo: ok` |
| Standing EL0 / `libctos` `fs_mkdir` sample | [`mkdir-libctos/`](mkdir-libctos/README.md), `libctos/`, `build.rs`, `src/mkdemo.rs` | same (FAT `/mkdemo`) | `libctos: mkdir-hi` / `libctos: mkdir-ok` / `mkdemo: ok` |
| Standing EL0 / `libctos` TCP echo sample | [`tcp-libctos/`](tcp-libctos/README.md), `libctos/`, `build.rs`, `src/tcpdemo.rs` | same (FAT `/tcpdemo`) | `libctos: tcp-hi` / `libctos: tcp-ok` / `tcpdemo: ok` |
| OS/app slot first cut (A9) | `target/hello-libctos.elf` on FAT `/hello` (`scripts/mkfat16.py --app`) | same (`qemu-smoke` publishes + attaches) | `slot: fat` / `slot: mapped` / `slot: ok` |
| memfs named buffers (optional) | `src/vfs.rs`; EL0 trampoline `/eprobe` in `src/syscall.rs` | same | `fs: create` / `fs: write` / `fs: read` / `fs: el0` / `fs: ok` |
| FAT16 `/probe` (optional) | `scripts/mkfat16.py`, `src/virtio.rs`, `src/fat.rs` | same (`qemu-smoke` attaches `-drive`) | `blk: ok` / `fat: ok` (`/probe` = `fat-hi`) |

Not claimed: POSIX, Linux ports, “secure OS,” EL0 isolated, PAN enable, taken SError. Product freestanding app hosting is **Verified** under ADR-048/052 — do not reopen that claim.

`hello-libctos` does **not** call `fs_*` (UART + one yield). `yield-libctos` does several cooperative `yield_now()` rounds (still no `fs_*`). `net-libctos` does `net_mac` / `net_ping` (ADR-068; kernel owns virtio-net). `udp-libctos` does `net_udp_dns` (ADR-071; same kernel NIC ownership; DNS probe bait only). `fs-libctos` does (create/open/read/write/close on `/memdemo` → memfs via ADR-058 `/mem` prefix). `fat-libctos` does (open/read/close on FAT `/probe` → FAT16 via ADR-058 `/`). The kernel `/eprobe` trampoline remains a separate Verified EL0 VFS trip.
