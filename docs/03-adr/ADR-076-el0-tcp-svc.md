# ADR-076 — EL0 thin TCP SVC + freestanding `tcp-libctos` sample

- Status: Accepted (EL0 TCP mile Verified when the serial / tests pass; BSD sockets / listen-accept / TLS / multi-connection table still non-goals)
- Date: 2026-09-17

## Context

[ADR-074](ADR-074-n5-thin-tcp.md) closed **N5** kernel-path thin TCP (`net: tcp-ok` via QEMU `guestfwd` `10.0.2.4:7`) and **deferred** the EL0 TCP SVC + sample. Public ABI today: FS SVCs 19–23 + 27, net 24–26 ([syscall.md](../framework/syscall.md)). Catalog samples exist for hello / VFS / FAT / yield / net / udp / mkdir.

Sponsor follow-up unlocks EL0 through the same quiet echo probe — one SVC, return 0 / fail. Do **not** implement BSD sockets, listen/accept, TLS, multi-connection tables, nested FAT/rmdir, or TCP CNTPCT in this PR.

FAT 8.3 path grammar: `/tcpdemo` fits (7 chars under the 8.3 name).

## Hypotheses

| ID | Option | Notes |
| --- | --- | --- |
| **H1 (chosen)** | Add `SYS_NET_TCP_ECHO` (28). No args. Kernel runs quiet ARP + thin TCP active-open + one payload vs guestfwd `10.0.2.4:7` (reuse ADR-074 helpers; distinct ephemeral port from N5). Return `0` on success, `u64::MAX` on fail. New crate `user/tcp-libctos` on FAT `/tcpdemo`. Print `libctos: tcp-hi` / `libctos: tcp-ok`. Kernel `tcpdemo: ok`. Keep kernel `net: tcp-ok`. | Tiny ABI; proves EL0 can trigger the existing thin TCP path. |
| **H2 (rejected)** | Ship BSD sockets / `connect`/`send`/`recv` / listen-accept this mile. | Honesty fail; locked product non-goals. |
| **H3 (rejected)** | Teach EL0 to program virtio-net MMIO or copy raw frames. | Kernel owns the NIC; SVCs only. |
| **H4 (rejected)** | Replace `/netdemo` / `/udpdemo` or another slot. | Breaks prior catalog greps. |
| **H5 (rejected)** | TCP CNTPCT / `perf: tcp-*` this PR. | Separate follow-up; out of scope. |

## Decision

1. **`SYS_NET_TCP_ECHO` = 28.** Document in [syscall.md](../framework/syscall.md). No args. Kernel `virtio::el0_tcp_echo` does quiet ARP + SYN/est/tx/rx vs guestfwd (no serial `net: tcp-*` markers on this quiet path). Return `0` on success, `u64::MAX` (`NET_ERR`) on fail. Unknown SVCs still park. Not sockets. Not a process ABI.
2. **`libctos`.** `ctos_net_tcp_echo` / `net_tcp_echo` + `SYS_NET_TCP_ECHO`. CRT must not issue reserved 0–2.
3. **Freestanding sample.** `user/tcp-libctos` on FAT `/tcpdemo` (8.3 `TCPDEMO`). Markers: `libctos: tcp-hi` / `libctos: tcp-ok` / `tcpdemo: ok`. Keep prior seven samples + kernel `net: tcp-ok` + all other smoke.
4. **Fail-closed smoke.** Grep the new markers; keep N1–N5 / N3.x / N4 / FAT / mkdir / samples. `#[test_case]` on the slot module + `el0_tcp_echo_probe`. Default `CTOS_FORCE_FAIL_TIMEOUT` is **180s** (was 120): ubuntu-24.04-arm force-fail rebuild+boot with eight FAT samples exceeded 120s; force-fail must still exit non-zero.
5. **Honesty.** Say “EL0 triggered the thin TCP guestfwd echo through `net_tcp_echo`” only when serial / tests pass. Do **not** say: BSD sockets, listen/accept, TLS, HTTP, “has networking / sockets OS,” EL0 virtio driver, Linux/POSIX, EL0 isolated, PAN, taken SError.
6. **NFR-10 text** revised in place (ID unchanged). Threat-model **v1.51**. Do not mint NFR-15+.

## Consequences

- Code: `src/syscall.rs`, `src/virtio.rs` (`el0_tcp_echo`), `libctos/`, `user/tcp-libctos/`, `src/tcpdemo.rs`, `build.rs`, `scripts/mkfat16.py`, `scripts/qemu-smoke.sh`.
- Docs: this ADR, syscall.md, honesty ledger, track-n, SUMMARY, what-can-run, apps-today, roadmap note, threat-model v1.51, `user/README.md`.
- Follow-ups (not this PR): TCP CNTPCT, richer socket ABI, listen-accept — new ADR + sponsor only.
