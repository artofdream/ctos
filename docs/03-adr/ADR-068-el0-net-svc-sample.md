# ADR-068 — Track N / N4 — EL0 net SVC + freestanding `net-libctos` sample

- Status: Accepted (sample + smoke Verified when serial / tests pass; product freestanding app hosting remains Verified under ADR-048/052 — do not reopen)
- Date: 2026-09-16

## Context

[ADR-063](ADR-063-network-foundation-scope.md) locked EL0 net ABI until **after** link bring-up Verified. [ADR-066](ADR-066-virtio-net-first-frame.md) closed **N1** (`net: ok`). [ADR-067](ADR-067-virtio-net-icmp-ping.md) closed **N2** (`net: ping-ok`). Catalog freestanding samples exist for hello / VFS / FAT / yield ([ADR-059](ADR-059-fs-libctos-sample.md) / [ADR-061](ADR-061-fat-libctos-sample.md) / [ADR-062](ADR-062-yield-libctos-sample.md)).

This mile (**N4**) unlocks a **tiny** EL0 net SVC surface and a fifth freestanding sample that proves net via SVCs only. Kernel still owns virtio-net. Do not break blk/FAT/slot/N1/N2/sample greps.

## Hypotheses

| ID | Option | Notes |
| --- | --- | --- |
| **H1 (chosen)** | Add `SYS_NET_MAC` (24) + `SYS_NET_PING` (25). New crate `user/net-libctos` on FAT `/netdemo`. Print `libctos: net-hi` / `libctos: net-mac` / `libctos: net-ok`. Kernel `netdemo: ok`. ADR + catalog + smoke. | Minimal ABI; proves MAC read + kernel-path ICMP from EL0 without a guest driver. |
| **H2 (rejected)** | Expose raw Ethernet frame TX/RX buffers to EL0 this mile. | Larger surface; prefer ping-as-proof. |
| **H3 (rejected)** | Ship BSD sockets / TCP/UDP / DHCP/DNS product. | Locked non-goals ([ADR-063](ADR-063-network-foundation-scope.md)). |
| **H4 (rejected)** | Teach EL0 to program virtio-net MMIO. | Honesty: kernel owns the NIC; SVCs only. |
| **H5 (rejected)** | Replace `/hello` or another slot. | Breaks A9 / catalog greps. |

## Decision

1. **SVC surface (document in [syscall.md](../framework/syscall.md)).**
   - `24` `net_mac` — `x0` = user buf, `x1` = length (≥6). Copy 6-byte guest MAC. Return `6`, or `0` if rejected / net not ready.
   - `25` `net_ping` — no args. Kernel does quiet ARP + ICMP echo vs SLIRP `10.0.2.2` (reuse N1/N2 helpers). Return `0` on success, `u64::MAX` on fail.
   - Unknown SVCs still park. Not sockets. Not a process ABI.

2. **Sample crate.** `user/net-libctos`: same `aarch64-ctos-user` / `linker.ld` / `build-std` as prior samples. `main` prints `libctos: net-hi`, calls `net_mac` then `libctos: net-mac`, calls `net_ping`, then `libctos: net-ok`.

3. **Publish.** `build.rs` builds and publishes `target/net-libctos.elf`. `scripts/mkfat16.py --app5` writes 8.3 `NETDEMO`. Keep `/hello` + `/fsdemo` + `/fatdemo` + `/yldemo` + their `*: ok` markers.

4. **Kernel probe.** `src/netdemo.rs` loads FAT `/netdemo`, `ERET`s via `loader::run_image_expecting` (last uart len 16 for `libctos: net-ok\n`). Serial `netdemo: fat` / `netdemo: mapped` / `netdemo: ok`. Runs after the ADR-062 yldemo probe. Requires N1/N2 virtio-net already brought up (`init_net`).

5. **Smoke.** Fail-closed greps for the new markers; preserve all existing greps (blk/fat/slot/net/ping/samples).

6. **Docs.** This ADR; honesty ledger evidence-only; `user/README.md`; [what-can-run.md](../overview/what-can-run.md); [apps-today.md](../framework/apps-today.md); [track-n.md](../04-roadmap/track-n.md); SUMMARY; threat-model **v1.43**.

7. **Honesty.** Do **not** claim: TCP/UDP product, BSD sockets, DHCP/DNS, Wi‑Fi, “has networking,” EL0 virtio driver, Linux/POSIX, EL0 isolated, or reopen ADR-048/052. CloudAgent HELD; tag agent-box / EVO-X2. No self-merge ([ADR-002](ADR-002-pr-identity-split.md)).

## Consequences

- Code: `libctos` + `sys.S`; `src/syscall.rs`; `src/virtio.rs` (`guest_mac` / `el0_icmp_ping`); `user/net-libctos/`; `src/netdemo.rs`; `build.rs`; `scripts/mkfat16.py`; `scripts/qemu-aarch64.sh`; `scripts/qemu-smoke.sh`; `src/main.rs`.
- Docs: this ADR + catalog / ledger / threat-model v1.43 / track-n N4.
- Follow-ups (not this PR): N3 sockets (locked); richer EL0 frame I/O only with new ADR + sponsor.
