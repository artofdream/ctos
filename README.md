# ctos

A minimal bare-metal x86_64 OS kernel in Rust. Learning and research project, tutorial-era stack (`bootloader` 0.9, custom target, VGA text).

This repo is hosted on **GitHub only** (`artofdream/ctos`). Issues, PRs, and reviews use `gh`. There is no GitLab tracker and no Pages publish step for these docs.

## Build and run

You need a **nightly** toolchain (see `rust-toolchain.toml`), `rust-src`, `llvm-tools-preview`, QEMU, and the `bootimage` cargo tool.

```bash
rustup toolchain install nightly
rustup component add rust-src llvm-tools-preview
cargo install bootimage
# Debian/Ubuntu: sudo apt-get install qemu-system-x86
```

`.cargo/config.toml` pins the custom target and the `bootimage runner`, so:

```bash
cargo build          # kernel + bootable image deps
cargo run            # boot the image in QEMU (needs QEMU on PATH)
cargo bootimage      # write target/x86_64-ctos/debug/bootimage-ctos.bin
```

QEMU boot was probed once in the 2026-09-08 cloud run (VGA dump at `0xb8e60` = `Hello World!`). That is not CI. Other machines stay Unknown until they run the same kind of probe. See the [ctos honesty ledger](docs/framework/honesty-ledger.md).

## Docs (document-first)

Start here before adding kernel features:

| Doc | What it is |
| --- | --- |
| [Product vision](docs/01-vision/product-vision.md) | What ctos is and is not |
| [Technical architecture](docs/02-architecture/technical-architecture.md) | `no_std`, bootloader 0.9, VGA stage, planned stages |
| [ADR-001](docs/03-adr/ADR-001-honesty-harness-for-ctos.md) | Why this repo uses a honesty/harness practice |
| [Roadmap](docs/04-roadmap/roadmap.md) | One milestone → one branch → one PR |
| [Harness map](docs/framework/formula.md) | Shared understanding, domain, outer harness — mapped to kernel work |
| [Honesty ledger](docs/framework/honesty-ledger.md) | Status words need a probe |
| [Antifragility SOP](docs/framework/antifragility.md) | Ratchet repeated failures into sensors |
| [AGENTS.md](AGENTS.md) | Session protocol and thin roles |
| [Second brain](research/README.md) | Vaults for session memory and handoffs |

## Honesty

Status words (Verified, Live, Done) are **claims**. Unprobed stays **Unknown**. Do not round Unknown up to Verified because the source exists or a PR is open.
