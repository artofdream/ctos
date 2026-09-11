# Overview (plain English)

ctos is a **learning** AArch64 kernel for QEMU `virt`. It is not a desktop, not POSIX, and not a “secure OS.” Status words need a probe in the [honesty ledger](honesty-ledger.md). This note does **not** invent latency, size, or “faster than” numbers. Measured markers live in the ledger; they are one environment each.

Vision: [product-vision.md](../01-vision/product-vision.md). Pillars: [pillars.md](pillars.md). Frozen IDs: [fr-nfr.md](../02-requirements/fr-nfr.md). Samples: [apps-today.md](apps-today.md). Porting: [building-or-porting.md](building-or-porting.md).

A docs website at https://ctos.artof.link is **Planned**. A Route 53 CNAME exists; this tree does not publish Pages. Do not claim that URL works.

## KPIs (what we actually measure)

These are **sensors**, not product SLOs. A missing marker is a fail. A printed number is not a budget.

### Performance ([NFR-07](../02-requirements/fr-nfr.md) / [performance.md](performance.md))

| What we watch | Probe (do not invent a target) |
| --- | --- |
| Physical counter moves | Serial `perf: cntpct delta=<n>` + `#[test_case]` |
| IRQ-to-handler spread on this virt guest | Serial `perf: irq-delta min=… max=… spread=… n=…` |
| Debug ELF byte size | Host `perf: elf-size bytes=<n>` (measurement, not “smaller is better”) |
| `kernel_main` after-MMU → after-init | Serial `perf: boot-delta ticks=<n>` |

QEMU TCG jitter is one lab. These are not Raspberry Pi numbers and not a published bench. Read the ledger row for the SHA you care about.

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

- **Can run (probed):** coop EL1 UART workers (`sched: task a/b/ok`); one-byte UART RX (`input: rx 0x41`); standing EL0 stub (`el0: standing` / `el0: restored`). A heartbeat/counter **variant** is the same shape — not in tree until a probe greps it.
- **Cannot run:** Linux ELF, shell, Python, network, filesystem, SMP, isolated userspace. Isolation / PAN / `.rodata`/`.data`/heap tear stay **Planned**. Filesystem stance: [filesystem.md](filesystem.md) (memfs → virtio-blk → FAT/xv6-like; none today).

## Building or porting

First-class page: [building-or-porting.md](building-or-porting.md). Short honesty:

- **Easiest** = in-tree `no_std` coop EL1 on `aarch64-ctos.json`, proven with `cargo` / `qemu-smoke` / `docker-smoke`.
- **POSIX / glibc** = not easy, not started.
- **SVC ABI + `libctos`** for freestanding EL0 = later **Planned** (standing dual-SVC is a stub, not a syscall table).

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
- Not POSIX, not multi-tenant, not a product runtime. No filesystem today ([filesystem.md](filesystem.md)).
- Isolation is **Planned**. Live identity `.text` after the boot stub is torn (ADR-020); `.rodata`/`.data`/heap stay
- Performance numbers are guest counter deltas, not a latency budget
- Docs website / custom domain is **Planned** (CNAME exists; Pages publish is a separate PR)
- Same-login cannot self-merge; a second identity has to land the PR

## Honesty

Do not say “secure OS,” “the kernel moved,” “EL0 isolated,” or “https://ctos.artof.link works.” Point at the [ledger](honesty-ledger.md) for any number you quote.
