# ADR-071 — Track N / N3.x — EL0 UDP SVC + freestanding `udp-libctos` sample

- Status: Accepted (sample + smoke Verified when serial / tests pass; product freestanding app hosting remains Verified under ADR-048/052 — do not reopen)
- Date: 2026-09-16

## Context

[ADR-070](ADR-070-n3-udp-transport.md) closed **N3** kernel-path UDP (`net: udp-ok`) and **deferred** EL0 UDP SVCs to **N3.x**. N4 ([ADR-068](ADR-068-el0-net-svc-sample.md)) already Verified tiny EL0 `net_mac` (24) / `net_ping` (25) + FAT `/netdemo`. Catalog samples exist for hello / VFS / FAT / yield / net.

Sponsor: proceed as proposed proactively (N3.x). Keep the surface **tiny and honest**. Do not ship BSD sockets, TCP product, or DHCP/DNS as product. Kernel still owns virtio-net.

## Hypotheses

| ID | Option | Notes |
| --- | --- | --- |
| **H1 (chosen)** | One combined `SYS_NET_UDP_DNS` (26) that mirrors the kernel N3 UDP DNS probe (quiet ARP + UDP query to SLIRP `10.0.2.3:53` + matching QR=1 reply). New crate `user/udp-libctos` on FAT `/udpdemo`. Print `libctos: udp-hi` / `libctos: udp-ok`. Kernel `udpdemo: ok`. | Tiny ABI; same probe bait as ADR-070; no send/recv buffers this mile. |
| **H2 (rejected)** | Pair `net_udp_send` / `net_udp_recv` with user buffers this mile. | Larger surface; H1 is enough to prove EL0 can trigger the existing kernel UDP path. |
| **H3 (rejected)** | Ship BSD sockets / TCP / DHCP/DNS product. | Locked non-goals ([ADR-063](ADR-063-network-foundation-scope.md) / [ADR-070](ADR-070-n3-udp-transport.md)). |
| **H4 (rejected)** | Teach EL0 to program virtio-net MMIO or copy raw frames. | Honesty: kernel owns the NIC; SVCs only. |
| **H5 (rejected)** | Replace `/netdemo` or another slot. | Breaks N4 / catalog greps. |

## Decision

1. **SVC surface (document in [syscall.md](../framework/syscall.md)).**
   - Keep `24` `net_mac` and `25` `net_ping` ([ADR-068](ADR-068-el0-net-svc-sample.md)).
   - `26` `net_udp_dns` — no args. Kernel does quiet ARP + one UDP DNS query vs SLIRP `10.0.2.3:53` (reuse N3 helpers). Return `0` on success, `u64::MAX` on fail.
   - Unknown SVCs still park. Not sockets. Not a process ABI. DNS remains **probe bait**, not a guest resolver.

2. **Sample crate.** `user/udp-libctos`: same `aarch64-ctos-user` / `linker.ld` / `build-std` as prior samples. `main` prints `libctos: udp-hi`, calls `net_udp_dns`, then `libctos: udp-ok`.

3. **Publish.** `build.rs` builds and publishes `target/udp-libctos.elf`. `scripts/mkfat16.py --app6` writes 8.3 `UDPDEMO`. Keep `/hello` + `/fsdemo` + `/fatdemo` + `/yldemo` + `/netdemo` + their `*: ok` markers.

4. **Kernel probe.** `src/udpdemo.rs` loads FAT `/udpdemo`, `ERET`s via `loader::run_image_expecting` (last uart len 16 for `libctos: udp-ok\n`). Serial `udpdemo: fat` / `udpdemo: mapped` / `udpdemo: ok`. Runs after the ADR-068 netdemo probe. Requires virtio-net already brought up (`init_net`).

5. **Smoke.** Fail-closed greps for the new markers; preserve all existing greps (N1–N4, udp-ok, blk/FAT/samples/perf).

6. **Docs.** This ADR; honesty ledger evidence-only; `user/README.md`; [what-can-run.md](../overview/what-can-run.md); [apps-today.md](../framework/apps-today.md); [track-n.md](../04-roadmap/track-n.md); SUMMARY; threat-model **v1.46**.

7. **Honesty.** Do **not** claim: BSD sockets, TCP product, DHCP/DNS as product, Wi‑Fi, “has networking / sockets OS,” EL0 virtio driver, Linux/POSIX, EL0 isolated, or reopen ADR-048/052. CloudAgent HELD; tag agent-box / EVO-X2. No self-merge ([ADR-002](ADR-002-pr-identity-split.md)).

## Consequences

- Code: `libctos` + `sys.S`; `src/syscall.rs`; `src/virtio.rs` (`el0_udp_dns`); `user/udp-libctos/`; `src/udpdemo.rs`; `build.rs`; `scripts/mkfat16.py`; `scripts/qemu-aarch64.sh`; `scripts/qemu-smoke.sh`; `src/main.rs`.
- Docs: this ADR + catalog / ledger / threat-model v1.46 / track-n N3.x.
- Follow-ups (not this PR): TCP / richer socket ABI / generic UDP send/recv — new ADR + sponsor only.
