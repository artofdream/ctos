# Overview (plain English)

**Core principles drive ctos.** Track A / Track B / A9 are subordinate and never override this list for speed or marketing. Full list: [principles.md](principles.md).

- Honesty ledger — Verified only with a probe
- Antifragility — fail-closed sensors + ratchets
- Security — threat model; probed mitigations only
- Performance — measure first; no invented benches
- Document-first / one milestone → one PR

ctos is a **learning** AArch64 kernel for QEMU `virt`. It is not a desktop, not POSIX, and not a “secure OS.” Status words need a probe in the [honesty ledger](honesty-ledger.md). This note does **not** invent latency, size, or “faster than” numbers. Measured markers live in the ledger; they are one environment each.

Vision: [product-vision.md](../01-vision/product-vision.md). Pillars: [pillars.md](pillars.md). Frozen IDs: [fr-nfr.md](../02-requirements/fr-nfr.md). Samples: [apps-today.md](apps-today.md). Porting: [building-or-porting.md](building-or-porting.md). Immutability: [immutability.md](immutability.md) (scoped only). Tracks: [A](../04-roadmap/track-a.md) / [B](../04-roadmap/track-b.md) (subordinate).

Site chapters (source of truth for the published book): [Overview](../overview/measure.md). HTTPS at https://ctos.artof.link is **Verified** (2026-09-11 after #30). See [website.md](../website.md).

## KPIs (what we actually measure)

These are **sensors**, not product SLOs. A missing marker is a fail. A printed number is not a budget.

### Performance ([NFR-07](../02-requirements/fr-nfr.md) / [performance.md](performance.md))

| What we watch | Probe (do not invent a target) |
| --- | --- |
| Physical counter moves | Serial `perf: cntpct delta=<n>` + `#[test_case]` |
| IRQ-to-handler spread on this virt guest | Serial `perf: irq-delta min=… max=… spread=… n=…` |
| Debug ELF byte size | Host `perf: elf-size bytes=<n>` (measurement, not “smaller is better”) |
| `kernel_main` after-MMU → after-init | Serial `perf: boot-delta ticks=<n>` |
| App-load after OS/app split (A9) | **Planned** `perf: app-load` CNTPCT + existing boot-delta. No marker today. |

QEMU TCG jitter is one lab. These are not Raspberry Pi numbers and not a published bench. Read the ledger row for the SHA you care about. A9 expected costs (boot/load, SVC, ASID/TTBR, later COW) vs steady EL0 compute: [performance.md](performance.md#osapp-slot-disconnect-a9--expected-shape-not-a-bench). **No Verified delta** — still one ELF.

### Antifragility ([NFR-05](../02-requirements/fr-nfr.md) / [antifragility.md](antifragility.md))

| What we watch | Probe |
| --- | --- |
| Hello + milestone strings still print | `scripts/qemu-smoke.sh` greps; missing string → fail |
| Tests stay fail-closed | `cargo test` exit 0; `--features force-fail` must be non-zero |
| Same failure twice | Becomes a sensor/gate, not another README paragraph |
| Failed history kept | Example: Docker `b2bbb99` Failed, then layout fix Verified — the Failed row stays |

Unprobed boot stays **Unknown**. File presence is not QEMU boot.

### Application-support scope

Do not say “applications run on ctos.” First-class samples and the cannot-run list live in [apps-today.md](apps-today.md). Porting stance: [building-or-porting.md](building-or-porting.md).

- **Can run (probed):** coop EL1 UART workers (`sched: task a/b/ok`); one-byte UART RX (`input: rx 0x41`); standing EL0 stub (`el0: standing` / `el0: restored`); a loaded `libctos` hello as a standing **task** until `exit` (`el0: task-ok`). A heartbeat/counter **variant** is the same shape — not in tree until a probe greps it.
- **Cannot run:** Linux ELF, shell, Python, network, POSIX disk apps, SMP, isolated userspace. Isolation / PAN enable / identity `.data`/heap tear stay **Planned**. Filesystem stance: [filesystem.md](filesystem.md) (memfs + read-only FAT16). Gaps to host apps + **containers: non-goal**: [host-apps.md](host-apps.md).

## Building or porting

First-class page: [building-or-porting.md](building-or-porting.md). Short honesty:

- **Easiest** = in-tree `no_std` coop EL1 on `aarch64-ctos.json`, proven with `cargo` / `qemu-smoke` / `docker-smoke`.
- **POSIX / glibc** = not easy, not started.
- **SVC ABI + `libctos` + guest `PT_LOAD` + standing-as-normal** for freestanding EL0 = A1 ABI + A2 CRT + A3 loader + A4 standing-task miles (dual-SVC stub still exists; not a syscall table; not isolation).

## OS image vs app payloads (A9)

The sponsor goal for “immutability” here is **not** a frozen kernel. It is to **disconnect OS updates from apps**: one OS image artifact, separate app payloads, so you can replace the kernel without rebuilding apps and replace apps without rebuilding the kernel.

- **First cut:** two host artifacts (`target/aarch64-ctos/debug/ctos` + `target/hello-libctos.elf`) and a FAT16 `/hello` load path ([ADR-030](../03-adr/ADR-030-os-app-slots.md), [A9 #48](https://github.com/artofdream/ctos/issues/48)). A2–A4 still embed a copy. Stance: [immutability.md](immutability.md).
- **Planned:** same app ELF on OS n and a documented prior OS (cross-update). Do not say “apps update independently” until that probe exists.
- Do not say “immutable OS.”
- **Performance:** `perf: app-load` measures the FAT load path. A Verified *delta* vs the embed stays Planned. Expected costs: boot/load, SVC, ASID/TTBR, optional later COW. Details: [performance.md](performance.md#osapp-slot-disconnect-a9--expected-shape-not-a-bench).

## Prerequisites

- Nightly Rust (`rust-toolchain.toml`), `rust-src`, `llvm-tools-preview`
- `qemu-system-aarch64` (Debian package is often `qemu-system-arm`)
- Optional: Docker **linux/arm64** (`./scripts/docker-smoke.sh`) — do not pin `linux/amd64`
- To *change* the kernel: one milestone → one branch → one GitHub PR; author ≠ merger ([ADR-002](../03-adr/ADR-002-pr-identity-split.md))

A machine that has not run `scripts/qemu-smoke.sh` (or Docker/GHA equivalent) has **Unknown** boot.

## Advantages

- Small, readable `no_std` tree on one ISA (AArch64 / QEMU `virt` / PL011 UART)
- Fail-closed smoke: serial markers, `cargo test`, force-fail
- Honesty ledger: Verified / Unknown / Planned / Failed, with a probe each
- Failures become sensors (layout miss, live-`.text` vtable yank, CRLF shebang)
- Three pillars named up front — security and speed still need probes

## Drawbacks

- QEMU `virt` only. No Raspberry Pi or board claim
- Not POSIX, not multi-tenant, not a product runtime. memfs + read-only FAT16 ([filesystem.md](filesystem.md)). Not a container host (**non-goal**, [ADR-029](../03-adr/ADR-029-containers-nongoal.md)); host `docker-smoke` ≠ guest Docker.
- Isolation is **Planned**. Live identity `.text` after the boot stub is torn (ADR-020); identity `.rodata` is torn (ADR-025); `.data`/heap stay; PAN enable stays Planned (`pan: absent` on `-cpu cortex-a57`)
- Scoped immutability only ([immutability.md](immutability.md)): RO+NX / WXN / live `.text` tear are probed. Absolute “immutable OS” is incompatible (heap/PTEs/devices must mutate). OS/app **slot first cut** (A9 / ADR-030) is two artifacts + FAT `/hello`. Cross-update stays Planned. A2–A4 still embed.
- Performance numbers are guest counter deltas, not a latency budget
- Docs website / custom domain: HTTPS serving the book is **Verified** ([website.md](../website.md))
- Same-login cannot self-merge; a second identity has to land the PR

## Honesty

Do not say “secure OS,” “the kernel moved,” “EL0 isolated,” “immutable OS,” or “apps update independently of the OS.” Point at the [ledger](honesty-ledger.md) for any number you quote. The live docs URL is a separate Verified row — do not invent other site claims.
