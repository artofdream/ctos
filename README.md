# ctos

A minimal bare-metal **AArch64** OS kernel in Rust. Learning and research project. Primary ISA is arm64 ([ADR-003](docs/03-adr/ADR-003-primary-isa-aarch64.md)): custom `aarch64-ctos.json`, QEMU `virt`, PL011 UART. x86_64 is not the primary path.

This repo is hosted on **GitHub only** (`artofdream/ctos`). Issues, PRs, and reviews use `gh`. There is no GitLab tracker.

Learning-kernel docs are built with **mdBook** and published with **GitHub Pages**. Production URL: [`https://ctos.artof.link`](https://ctos.artof.link) — **Verified** after #30 (main deploy [34653046584](https://github.com/artofdream/ctos/actions/runs/34653046584) + HTTPS 200 + Driving principles). Route 53 CNAME remains in place (zone `Z1178AFMV41RWP`, account `737290977112`). `https://artofdream.github.io/ctos` (no trailing slash) 301s to the custom domain. Overview: [what can run today](docs/overview/what-can-run.md), [prerequisites](docs/overview/prerequisites.md), [limits](docs/overview/limits.md). DNS/Pages: [docs/website.md](docs/website.md).

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
./scripts/qemu-smoke.sh   # build + hello + paging + heap + two-task sched + W^X + guards + RO+NX + EL0 first mile + no-kernel-read + standing + ASID + TTBR1 private + high-VA exec + identity-tear + vtable reloc + live .text tear + CNTPCT + boot-delta + IRQ-delta + host ELF size + timer tick + injected UART RX + BRK + fatal nested serial + cargo test + force-fail
cargo test                # #[test_case] including VBAR/BRK/stacks/guards/RO+NX/timer/IRQ-delta/empty RX/MMU/frames/heap/sched/CNTPCT/boot-delta/W^X/EL0/standing/ASID/TTBR1 private + high-VA exec + identity-tear + vtable reloc + live .text; QEMU exits 0 via ARM semihosting
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

cts-ai `./scripts/docker-smoke.sh` (linux/arm64) is **Verified** on `main` `e80dc93` (Merge PR #28 / ADR-020): 50 tests, `ident: reloc n=12`, live pages=37, force-fail ok. Earlier Docker Verified: `24d94e6` (#27 / ADR-019), `b0f0ee5` (#24–#26), and `71ee15f` (layout L3). The first 2026-09-09 Docker pass after the LF / `build-essential` / ROM ratchets is still a ledger row. Keep the `b2bbb99` Failed row. This is not a Raspberry Pi port.

## Docs website (mdBook)

```bash
./scripts/docs-build.sh   # installs mdBook 0.5.4 + mdbook-mermaid 0.17.1 if needed, then `mdbook build`
mdbook serve              # optional: http://localhost:3000
```

A green local build is a **generator** probe only. The live custom domain is a **separate** ledger row (**Verified** after #30). See [docs/website.md](docs/website.md).

## Docs (document-first)

Start here before adding kernel features:

| Doc | What it is |
| --- | --- |
| [KPIs / how we measure](docs/overview/measure.md) | Performance, stability, honest app-support scope |
| [What can run today](docs/overview/what-can-run.md) | UART workers, RX echo, EL0 stub — not Linux/Python/net |
| [Building or porting](docs/overview/porting.md) | In-tree `no_std` today; no easy POSIX port |
| [Filesystem (Planned)](docs/overview/filesystem.md) | No FS today; memfs then virtio-blk; not FAT-supported |
| [Hosting apps / containers](docs/overview/hosting-apps.md) | Gaps table; containers: no |
| [Prerequisites](docs/overview/prerequisites.md) | Nightly Rust + QEMU virt; Pages not required for kernel work |
| [Advantages](docs/overview/advantages.md) | Document-first, probed claims, pillars as NFRs |
| [Drawbacks / limits](docs/overview/limits.md) | Learning kernel; identity stub; PAN unclaimed; no net/DMA |
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
| [ADR-017](docs/03-adr/ADR-017-ttbr1-high-el1-exec.md) | EL1 fetch from TTBR1 RAM alias |
| [ADR-018](docs/03-adr/ADR-018-identity-teardown-first-cut.md) | Identity-tear first cut (split tables + one torn text page) |
| [ADR-019](docs/03-adr/ADR-019-identity-text-range-tear.md) | High-VA continuation + 16 KiB dedicated identity text range |
| [ADR-020](docs/03-adr/ADR-020-identity-fnptr-reloc.md) | High-VA vtable rewrite + live identity `.text` tear |
| [Roadmap](docs/04-roadmap/roadmap.md) | One milestone → one branch → one PR |
| [Harness map](docs/framework/formula.md) | Shared understanding, domain, outer harness — mapped to kernel work |
| [Honesty ledger](docs/framework/honesty-ledger.md) | Status words need a probe |
| [Three pillars](docs/framework/pillars.md) | Antifragility, security, performance (NFR-05 / NFR-10 / NFR-07) |
| [Antifragility SOP](docs/framework/antifragility.md) | Ratchet repeated failures into sensors |
| [Security](docs/framework/security.md) | Threat-model v1.8; not a “secure OS” claim |
| [EL0](docs/framework/el0.md) | First mile + standing + TTBR1 + high-VA exec + identity `.text` range + live `.text` tear; isolation Planned |
| [Performance](docs/framework/performance.md) | CNTPCT + IRQ-delta + host ELF size + boot-delta; no fake benches |
| [AGENTS.md](AGENTS.md) | Session protocol and thin roles |
| [Docs website + DNS](docs/website.md) | mdBook + Pages; `https://ctos.artof.link` HTTPS Verified after #30 |
| [Second brain](research/README.md) | Vaults for session memory and handoffs |

## Honesty

Status words (Verified, Live, Done) are **claims**. Unprobed stays **Unknown**. Do not round Unknown up to Verified because the source exists or a PR is open.
