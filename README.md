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
./scripts/qemu-smoke.sh   # build + require hello on serial + cargo test + force-fail
cargo test                # two #[test_case]; QEMU exits 0 via ARM semihosting
```

### Docker (cts-ai: Windows ARM64 → linux/arm64)

Do **not** pass `--platform linux/amd64`. The image is `ubuntu:24.04` (multi-arch) plus nightly Rust and `qemu-system-aarch64`.

```bash
./scripts/docker-smoke.sh
# or
docker build -t ctos-smoke .
docker run --rm ctos-smoke
# optional: docker compose run --rm smoke
```

Docker-on-cts-ai is Unknown until that engine runs it. This is not a Raspberry Pi port.

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
| [Roadmap](docs/04-roadmap/roadmap.md) | One milestone → one branch → one PR |
| [Harness map](docs/framework/formula.md) | Shared understanding, domain, outer harness — mapped to kernel work |
| [Honesty ledger](docs/framework/honesty-ledger.md) | Status words need a probe |
| [Antifragility SOP](docs/framework/antifragility.md) | Ratchet repeated failures into sensors |
| [AGENTS.md](AGENTS.md) | Session protocol and thin roles |
| [Second brain](research/README.md) | Vaults for session memory and handoffs |

## Honesty

Status words (Verified, Live, Done) are **claims**. Unprobed stays **Unknown**. Do not round Unknown up to Verified because the source exists or a PR is open.
