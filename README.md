# ctos

A minimal bare-metal **AArch64** OS kernel in Rust. Learning and research project. Primary ISA is arm64 ([ADR-003](docs/03-adr/ADR-003-primary-isa-aarch64.md)): custom `aarch64-ctos.json`, QEMU `virt`, PL011 UART. x86_64 is not the primary path.

This repo is hosted on **GitHub only** (`artofdream/ctos`). Issues, PRs, and reviews use `gh`. There is no GitLab tracker and no Pages publish step for these docs.

## Build and run

You need a **nightly** toolchain (see `rust-toolchain.toml`), `rust-src`, `llvm-tools-preview`, and `qemu-system-aarch64`.

```bash
rustup toolchain install nightly
rustup component add rust-src llvm-tools-preview
# Debian/Ubuntu: sudo apt-get install qemu-system-arm
# (package name may be qemu-system-arm; the binary is qemu-system-aarch64)
```

`.cargo/config.toml` pins the custom target and a QEMU runner, so:

```bash
cargo build          # freestanding aarch64 kernel ELF
cargo run            # boot the ELF in qemu-system-aarch64 -machine virt
```

The ELF lands at `target/aarch64-ctos/debug/ctos`. QEMU serial Hello World was probed once in the 2026-09-08 cloud run (`qemu-system-aarch64` 8.2.2, `-machine virt`). That is not CI. Other machines stay Unknown until they run the same kind of probe. See the [ctos honesty ledger](docs/framework/honesty-ledger.md).

```bash
./scripts/qemu-smoke.sh   # build + hello + paging + heap + two-task sched + W^X + guards + RO+NX + EL0 first mile + no-kernel-read + standing + ASID + TTBR1 private + high-VA exec + CNTPCT + boot-delta + IRQ-delta + host ELF size + timer tick + injected UART RX + BRK + fatal nested serial + cargo test + force-fail
cargo test                # #[test_case] including VBAR/BRK/stacks/guards/RO+NX/timer/IRQ-delta/empty RX/MMU/frames/heap/sched/CNTPCT/boot-delta/W^X/EL0/standing/ASID/TTBR1 private + high-VA exec; QEMU exits 0 via ARM semihosting
```

### Docker (cts-ai: Windows ARM64 → linux/arm64)

Do **not** pass `--platform linux/amd64`. The image is `ubuntu:24.04` (multi-arch) plus nightly Rust, `build-essential` (host `cc` for `compiler_builtins` / build-std), `qemu-system-arm`, `qemu-efi-aarch64`, `ipxe-qemu` (`efi-virtio.rom`), and `python3` (UART RX inject). Shell scripts are LF-only (`.gitattributes`); a CRLF shebang makes `docker run` fail with `no such file or directory`.

```bash
./scripts/docker-smoke.sh
# or
docker build -t ctos-smoke .
docker run --rm ctos-smoke
# optional: docker compose run --rm smoke
```

cts-ai `docker build` + `docker run --rm ctos-smoke` (linux/arm64, 2026-09-09) is **Verified** after the LF / `build-essential` / ROM ratchets (see the honesty ledger). This is not a Raspberry Pi port.

## Docs (document-first)

Start here before adding kernel features:

| Doc | What it is |
| --- | --- |
| [Product vision](docs/01-vision/product-vision.md) | What ctos is and is not |
| [FR / NFR](docs/02-requirements/fr-nfr.md) | Frozen functional and non-functional IDs (ISA text revised under ADR-003) |
| [Technical architecture](docs/02-architecture/technical-architecture.md) | `no_std`, QEMU `virt`, UART stage, planned stages |
| [ADR-001](docs/03-adr/ADR-001-honesty-harness-for-ctos.md) | Why this repo uses a honesty/harness practice |
| [ADR-002](docs/03-adr/ADR-002-pr-identity-split.md) | Author ≠ merger; `artofdream` vs `cursor[bot]` |
| [ADR-003](docs/03-adr/ADR-003-primary-isa-aarch64.md) | Primary ISA is AArch64 |
| [ADR-004](docs/03-adr/ADR-004-el1-vbar-brk.md) | EL1 `VBAR_EL1` + resumable `BRK` |
| [ADR-005](docs/03-adr/ADR-005-fatal-exception-stack.md) | Dedicated exception + fatal stacks (FR-07) |
| [ADR-006](docs/03-adr/ADR-006-gicv2-generic-timer.md) | GICv2 + EL1 physical timer (FR-08) |
| [ADR-007](docs/03-adr/ADR-007-pl011-uart-rx.md) | PL011 UART RX as virt input (FR-08 / M6) |
| [ADR-008](docs/03-adr/ADR-008-identity-map-frame-allocator.md) | Identity map + bump frame allocator (FR-09 / M7) |
| [ADR-009](docs/03-adr/ADR-009-first-fit-heap.md) | First-fit `GlobalAlloc` heap (FR-10 / M8) |
| [ADR-010](docs/03-adr/ADR-010-cooperative-rr-el1.md) | Cooperative round-robin on EL1 (FR-11 / M9) |
| [ADR-011](docs/03-adr/ADR-011-three-pillars.md) | Three pillars: antifragility, security, performance |
| [ADR-012](docs/03-adr/ADR-012-wx-nx-heap-stacks.md) | W^X: NX heap + cooperative stacks |
| [ADR-013](docs/03-adr/ADR-013-el0-isolation-direction.md) | EL0 isolation direction + first mile + standing (isolation Planned) |
| [ADR-014](docs/03-adr/ADR-014-linker-stack-guard-pages.md) | Unmapped 4 KiB holes under linker stacks |
| [ADR-015](docs/03-adr/ADR-015-ro-nx-text-data.md) | RO+NX text/data split (`SCTLR.WXN`) |
| [ADR-016](docs/03-adr/ADR-016-ttbr1-private-page.md) | TTBR1 kernel-private page (first cut) |
| [ADR-017](docs/03-adr/ADR-017-ttbr1-high-el1-exec.md) | EL1 fetch from TTBR1 RAM alias (identity teardown Planned) |
| [Roadmap](docs/04-roadmap/roadmap.md) | One milestone → one branch → one PR |
| [Harness map](docs/framework/formula.md) | Shared understanding, domain, outer harness — mapped to kernel work |
| [Honesty ledger](docs/framework/honesty-ledger.md) | Status words need a probe |
| [Three pillars](docs/framework/pillars.md) | Antifragility, security, performance (NFR-05 / NFR-10 / NFR-07) |
| [Antifragility SOP](docs/framework/antifragility.md) | Ratchet repeated failures into sensors |
| [Security](docs/framework/security.md) | Threat-model v1.5; not a “secure OS” claim |
| [EL0](docs/framework/el0.md) | First mile + standing + TTBR1 first cut + high-VA exec; isolation Planned |
| [Performance](docs/framework/performance.md) | CNTPCT + IRQ-delta + host ELF size + boot-delta; no fake benches |
| [AGENTS.md](AGENTS.md) | Session protocol and thin roles |
| [Second brain](research/README.md) | Vaults for session memory and handoffs |

## Honesty

Status words (Verified, Live, Done) are **claims**. Unprobed stays **Unknown**. Do not round Unknown up to Verified because the source exists or a PR is open.
