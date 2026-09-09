# ctos honesty ledger

Status words are claims. Each row needs a **probe**. Unprobed = **Unknown**. Never round Unknown up to Verified because the file exists, a PR is open, or a README teaches `cargo run`.

Allowed flags: **Verified** (probe passed), **Unknown** (no probe or probe blocked), **Planned** (not built yet), **Failed** (probe ran and lost).

| Claim | Probe | Status | Notes |
| --- | --- | --- | --- |
| UART hello source present (`println!("Hello World!");` + PL011 writer) | Read `src/main.rs` and `src/uart.rs` on this branch | Verified | Source inspection. Boot is a separate row. |
| `.cargo/config.toml` present (build-std, `aarch64-ctos.json`, qemu runner, `json-target-spec`) | Read `.cargo/config.toml` | Verified | Presence. Build success is the next row. |
| `cargo build` for `aarch64-ctos.json` | `cargo +nightly build` on 2026-09-08 (`rustc` 1.100.0-nightly `cea272fa3`) | Verified | First attempt failed: `invalid aarch64 ABI combination` until the JSON had both `"abi": "softfloat"` and `"rustc-abi": "softfloat"`. Then the ELF built (`target/aarch64-ctos/debug/ctos`, AArch64, entry `0x40080000`). |
| QEMU aarch64 serial shows Hello World | `qemu-system-aarch64` 8.2.2, `-machine virt -cpu cortex-a57 -display none -serial stdio -kernel target/aarch64-ctos/debug/ctos`; `timeout 4` | Verified | Serial printed `Hello World!` then the VM was killed (exit 124). Same string on `-machine virt,gic-version=3` and `-cpu max`. One cloud environment, not CI. Not a Raspberry Pi probe. |
| x86_64 VGA / bootimage path | Historical probes on 2026-09-08 (PR #2) | Historical | Path **removed** by ADR-003. Those Verified rows do not apply to this tree. |
| CI on GitHub | Workflow file exists **and** a run is green | Verified | `.github/workflows/smoke.yml`: push run [34285784458](https://github.com/artofdream/ctos/actions/runs/34285784458) success (`ubuntu-24.04` 52s, `ubuntu-24.04-arm` 1m9s). PR run [34285786641](https://github.com/artofdream/ctos/actions/runs/34285786641) success. `ubuntu-24.04-arm` label is available on this repo. |
| Docker smoke (`Dockerfile` / `scripts/docker-smoke.sh`) | cts-ai Docker Desktop `linux/arm64`, 2026-09-09: `docker build -t ctos-smoke .` && `docker run --rm ctos-smoke` → exit 0 | Verified | After three Failed runs, ratchets landed in git and the full smoke passed on cts-ai: serial `Hello World!` (hello QEMU timeout 124, expected); `cargo test` `[ok]`; force-fail exit 1; `qemu-smoke: ok`. Failures that were ratcheted: (1) CRLF shebang → `exec ./scripts/qemu-smoke.sh: no such file or directory` (`.gitattributes` `*.sh`/`Dockerfile` `eol=lf`, image `sed`, `CMD bash`); (2) `linker cc not found` on `compiler_builtins` (`build-essential`); (3) `failed to find romfile "efi-virtio.rom"` (`qemu-efi-aarch64` + `ipxe-qemu`). One sponsor host, not CI and not this cloud VM (still no Docker engine here). |
| Second-brain vaults (`research/`) | Paths exist; README explains Procedure / Correction / Relationship / Daily Brief | Verified | Structure present. Not a claim that vaults are richly filled. Optional Obsidian UI is structure-only (PR #3). |
| Thin `ctos-*` roles | `AGENTS.md` + `.cursor/skills/ctos-*/SKILL.md` exist | Verified | Four roles. No `aea-*` names. |
| Frozen FR/NFR IDs (`FR-01`–`FR-15`, `NFR-01`–`NFR-14`) | Read [docs/02-requirements/fr-nfr.md](../02-requirements/fr-nfr.md); IDs present; ISA **text** revised under [ADR-003](../03-adr/ADR-003-primary-isa-aarch64.md) | Verified | **Frozen IDs.** Text now AArch64/UART. Do not invent extra FR/NFR IDs in chat. File presence is not a claim that every Now row is implemented. |
| PR identity split (author ≠ merger; `artofdream` vs `cursor[bot]`) | Read [ADR-002](../03-adr/ADR-002-pr-identity-split.md), `AGENTS.md`, `.cursor/rules/pr-identity-no-self-merge.mdc` | Verified | Docs present. Principle reused from Café Fausse `pr-coordinator` (identity only). **`cursor[bot]` merge / App APPROVE on this repo:** Unknown until probed on `artofdream/ctos`. |
| Primary ISA is AArch64 | Read [ADR-003](../03-adr/ADR-003-primary-isa-aarch64.md); `x86_64-ctos.json` / `src/vga_buffer.rs` absent | Verified | Decision + file tree. QEMU boot is a separate row. |
| Integration tests / QEMU test exit (M2) | `cargo +nightly test` → QEMU virt + `-semihosting`; two `#[test_case]`; QEMU host exit 0 | Verified | 2026-09-08 cloud: serial showed `Running 2 tests` / `[ok]`; process exit 0. |
| Fail-closed test panic | `cargo +nightly test --features force-fail` | Verified | 2026-09-08 cloud: panic `force-fail`, QEMU/host exit 1. |
| `scripts/qemu-smoke.sh` | Ran on this cloud VM | Verified | Hello string present (timeout 124), `cargo test` 0, force-fail 1. Not CI. Not Docker. |

## How to update

1. Run or cite the probe (command, file path + revision, or `gh run` URL).
2. Change only the rows you probed.
3. If you could not run QEMU, leave boot **Unknown** and say so in the PR. A prior Verified row is one environment and one boot path; do not copy the 2026-09-08 x86 VGA probe forward.
