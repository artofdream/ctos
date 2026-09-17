# Performance (NFR-07 / ADR-011)

Performance is a first-class pillar, not a Later learning-only note. That does **not** authorize fake benches.

## Rules

1. A latency, cycle, or “faster than” sentence is a **claim**. It needs a probe in the [honesty ledger](honesty-ledger.md).
2. Optimize only after a probe shows a cost. Do not rewrite the scheduler or heap “for speed” on a hunch.
3. QEMU `virt` numbers are one environment. They are not a Raspberry Pi probe and not a published SPEC run.

## Baseline probe (this tree)

`CNTPCT_EL0` is already used to timeout the M5 timer observe window. The pillars ratchet (when landed) measures a **fixed trivial loop**:

- Serial marker `perf: cntpct delta=<n>` (fail closed on `perf: probe missed`).
- `#[test_case]` asserts the counter advanced.

That probe proves the physical counter is readable and moves. It does **not** claim a microsecond budget, interrupt latency, or a comparison to other kernels.

## IRQ-to-handler probe (this tree)

When CNTP fires, the handler records `CNTPCT − CNTP_CVAL` before rearm. After several ticks the hello kernel prints:

- Serial marker `perf: irq-delta min=<a> max=<b> spread=<b-a> n=<n>` (fail closed on `perf: irq-delta missed`).
- `#[test_case]` asserts samples exist and `max >= min`.

That is a **spread of IRQ-to-handler counter deltas on this QEMU virt guest**. It is not a latency budget, not “faster than X,” and not a published bench. QEMU TCG jitter is one environment.

## Host debug ELF size (NFR-08, this tree)

`scripts/qemu-smoke.sh` prints the host byte size of `target/aarch64-ctos/debug/ctos` after `cargo build`:

- Host marker `perf: elf-size bytes=<n>` (fail closed if missing or `< 4096`).
- This is a **measurement**, not a size budget and not a “smaller is better” claim.

It does not time QEMU boot.

## Boot-to-ready CNTPCT (NFR-08, this tree)

`kernel_main` samples `CNTPCT_EL0` after `paging::init` (MMU + D-cache on) and again after `Hello World!` (init complete):

- Serial marker `perf: boot-delta ticks=<n>` (fail closed on `perf: boot-delta missed`).
- `#[test_case]` asserts a sample exists and the counter advanced.

That is **kernel_main-entry to after-init** on this QEMU virt guest. It is not a latency budget, not QEMU startup time, not a published bench, and not criterion.

## OS/app slot disconnect (A9) — compared pair (QEMU TCG lab)

[A9 #48](https://github.com/artofdream/ctos/issues/48) / [ADR-030](../03-adr/ADR-030-os-app-slots.md) loads a **separate app payload** from FAT `/hello` after the OS image ([immutability.md](immutability.md), site [measure.md](../overview/measure.md)). The load path prints `perf: app-load ticks=<n>`. [ADR-046](../03-adr/ADR-046-slot-perf-delta.md) adds a **probe-only** linked-in trip of the same `hello-libctos.elf` bytes (`include_bytes!` in `src/slot.rs` only) that prints `perf: embed-load ticks=<n>`, then an honest pair `perf: slot-delta app=<a> embed=<b>`. Production A9 stays FAT-only (`slot: embed` must never print). That is a **measurement pair**, not a bench and not a percent.

| Class | What we measure | Honesty |
| --- | --- | --- |
| **FAT path** | VFS/FAT read + ELF parse + user map + `ERET` | `perf: app-load ticks=<n>` |
| **Probe-only embed** | Same bytes already in the kernel image; parse + map + `ERET` (no FAT) | `perf: embed-load ticks=<n>` — not `slot: embed` |
| **Pair** | Raw tick counts on one boot | `perf: slot-delta app=<a> embed=<b>`. No “faster/slower.” No invented percent. QEMU TCG jitter. |
| **Operational** | Smaller OS updates without rebuilding apps | Operational ≠ measured latency. A2–A4 stay FAT-only ([ADR-032](../03-adr/ADR-032-track-a-leftovers.md)). |
| **Gate** | Keep `perf: boot-delta`. Fail closed on missing `perf: app-load` / `perf: embed-load` / `perf: slot-delta` | Verified only when the ledger has serial evidence on a named tip. |

QEMU TCG jitter is still one lab. No `criterion` crate. No “slot disconnect is free.”


## FAT16 vs memfs write (ADR-051) — compared pair (QEMU TCG lab)

[ADR-050](../03-adr/ADR-050-fat16-write.md) writes a small payload through the thin VFS onto the FAT16 volume. [ADR-051](../03-adr/ADR-051-fat-memfs-write-cntpct.md) samples `CNTPCT` around that **single** `vfs::write` (`perf: fat-write ticks=<n>`), then on the same boot times a memfs `vfs::write` of the **same** bytes (`perf: memfs-write ticks=<n>`), and prints `perf: fs-write-delta fat=<a> memfs=<b>`. That is a **measurement pair**, not a bench and not a percent.

| Class | What we measure | Honesty |
| --- | --- | --- |
| **FAT write** | One VFS write of the `/probe` rewrite payload (`fat-wr`) | `perf: fat-write ticks=<n>` |
| **Memfs write** | One VFS write of the same bytes on `/mwprobe` | `perf: memfs-write ticks=<n>` |
| **Pair** | Raw tick counts on one boot | `perf: fs-write-delta fat=<a> memfs=<b>`. No “faster/slower.” No invented percent. QEMU TCG jitter. |
| **Gate** | Keep `fat: write` / `fat: ok` / A9 markers. Fail closed on missing pair markers | Verified only when the ledger has serial evidence on a named tip. |

QEMU TCG jitter is still one lab. No `criterion` crate. No latency SLA for FAT write.

## FAT16 vs memfs read (ADR-065) — compared pair (QEMU TCG lab)

The guest already reads known FAT16 `/probe` bytes through the thin VFS. [ADR-065](../03-adr/ADR-065-fat-memfs-read-cntpct.md) samples `CNTPCT` around that **single** `vfs::read` (`perf: fat-read ticks=<n>`), then on the same boot seeds memfs `/mrprobe` with the **same** bytes and times `vfs::read` (`perf: memfs-read ticks=<n>`), and prints `perf: fs-read-delta fat=<a> memfs=<b>`. That is a **measurement pair**, not a bench and not a percent. Symmetric to [ADR-051](../03-adr/ADR-051-fat-memfs-write-cntpct.md) (write).

| Class | What we measure | Honesty |
| --- | --- | --- |
| **FAT read** | One VFS read of `/probe` (`fat-hi`) | `perf: fat-read ticks=<n>` |
| **Memfs read** | One VFS read of the same bytes on `/mrprobe` | `perf: memfs-read ticks=<n>` |
| **Pair** | Raw tick counts on one boot | `perf: fs-read-delta fat=<a> memfs=<b>`. No “faster/slower.” No invented percent. QEMU TCG jitter. |
| **Gate** | Keep `fat: read` / `fat: ok` / ADR-051 write pair / A9 markers. Fail closed on missing pair markers | Verified only when the ledger has serial evidence on a named tip. |

QEMU TCG jitter is still one lab. No `criterion` crate. No latency SLA for FAT read.



## Net-ping path CNTPCT (ADR-069) — single path (QEMU TCG lab)

[ADR-068](../03-adr/ADR-068-el0-net-svc-sample.md) EL0 `net_ping` (SVC 25) already runs a quiet kernel-owned ARP + ICMP echo vs SLIRP `10.0.2.2`. [ADR-069](../03-adr/ADR-069-net-ping-cntpct.md) samples `CNTPCT` around that **full quiet path** and prints `perf: net-ping ticks=<n>`. That is a **measurement**, not a bench and not a latency SLA. Related optional N2 crumbs (`perf: net-icmp-tx` / `perf: net-icmp-rx`) stay as ADR-067 lab notes.

| Class | What we measure | Honesty |
| --- | --- | --- |
| **EL0 net_ping** | Quiet ARP + ICMP echo RTT as `SYS_NET_PING` / `el0_icmp_ping` | `perf: net-ping ticks=<n>` |
| **Gate** | Smoke greps the **prefix** `perf: net-ping` (not an exact tick count). Reject faster/slower/percent. Keep N1/N2/N4 / FAT / slot markers. | Verified only when the ledger has serial evidence on a named tip. |

QEMU TCG jitter is still one lab. Tick values **vary** — do not invent a budget. No `criterion` crate. No sockets / TCP/UDP this mile.

## UDP DNS path CNTPCT (ADR-072) — single path (QEMU TCG lab)

[ADR-071](../03-adr/ADR-071-n3x-el0-udp-svc.md) EL0 `net_udp_dns` (SVC 26) already runs a quiet kernel-owned ARP + UDP DNS query vs SLIRP `10.0.2.3:53`. [ADR-072](../03-adr/ADR-072-udp-dns-cntpct.md) samples `CNTPCT` around that **full quiet path** and prints `perf: udp-dns ticks=<n>`. That is a **measurement**, not a bench and not a latency SLA. Same pattern as ADR-069 on `net_ping` — **EL0 SVC surface**, not a separate kernel-only N3 timer.

| Class | What we measure | Honesty |
| --- | --- | --- |
| **EL0 net_udp_dns** | Quiet ARP + UDP DNS RTT as `SYS_NET_UDP_DNS` / `el0_udp_dns` | `perf: udp-dns ticks=<n>` |
| **Gate** | Smoke greps the **prefix** `perf: udp-dns` (not an exact tick count). Reject faster/slower/percent. Keep N1/N2/N3/N3.x/N4 / ADR-069 / FAT / slot markers. | Verified only when the ledger has serial evidence on a named tip. |

QEMU TCG jitter is still one lab. Tick values **vary** — do not invent a budget. No `criterion` crate. No TCP / FAT mkdir this mile.

## Thin TCP echo path CNTPCT (ADR-078) — single path (QEMU TCG lab)

[ADR-076](../03-adr/ADR-076-el0-tcp-svc.md) EL0 `net_tcp_echo` (SVC 28) already runs a quiet kernel-owned ARP + thin TCP active-open + one payload vs QEMU `guestfwd` `10.0.2.4:7`. [ADR-078](../03-adr/ADR-078-tcp-echo-cntpct.md) samples `CNTPCT` around that **full quiet path** and prints `perf: tcp-echo ticks=<n>`. That is a **measurement**, not a bench and not a latency SLA. Same pattern as ADR-069 / ADR-072 — **EL0 SVC surface**, not a separate kernel-only N5 timer. Kernel `net: tcp-ok` markers stay as ADR-074 transport proof.

| Class | What we measure | Honesty |
| --- | --- | --- |
| **EL0 net_tcp_echo** | Quiet ARP + thin TCP guestfwd RTT as `SYS_NET_TCP_ECHO` / `el0_tcp_echo` | `perf: tcp-echo ticks=<n>` |
| **Gate** | Smoke greps the **prefix** `perf: tcp-echo` (not an exact tick count). Reject faster/slower/percent. Keep N1–N5 / N5.x / ADR-069/072 / FAT / slot markers. | Verified only when the ledger has serial evidence on a named tip. |

QEMU TCG jitter is still one lab. Tick values **vary** — do not invent a budget. No `criterion` crate. No listen/accept / sockets / TLS this mile.

## Later probes (Planned)

- A tighter “first instruction of `_start`” sample if someone maps a `.data` slot that BSS-clear will not wipe.

Do not add a host `criterion` crate or a “bench.yml” that prints invented numbers.
