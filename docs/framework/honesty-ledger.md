# ctos honesty ledger

Status words are claims. Each row needs a **probe**. Unprobed = **Unknown**. Never round Unknown up to Verified because the file exists, a PR is open, or a README teaches `cargo run`.

Allowed flags: **Verified** (probe passed), **Unknown** (no probe or probe blocked), **Planned** (not built yet), **Failed** (probe ran and lost).

| Claim | Probe | Status | Notes |
| --- | --- | --- | --- |
| VGA hello source present (`println!("Hello World!");` + `vga_buffer` writer) | Read `src/main.rs` and `src/vga_buffer.rs` on this branch | Verified | Source inspection. Boot is a separate row. |
| `.cargo/config.toml` present (build-std, `x86_64-ctos.json`, bootimage runner, `json-target-spec`) | Read `.cargo/config.toml` | Verified | Presence. Build success is the next row. |
| `cargo build` for `x86_64-ctos.json` | `cargo +nightly build` on 2026-09-08 (`rustc` 1.100.0-nightly `cea272fa3`) | Verified | First attempts failed: missing `json-target-spec`, string `target-pointer-width`, missing `rustc-abi`, stale `data-layout`. Ratcheted into config + target JSON; then the build finished. |
| `cargo bootimage` writes `bootimage-ctos.bin` | `cargo +nightly bootimage` → `target/x86_64-ctos/debug/bootimage-ctos.bin` (179200 bytes) | Verified | Same cloud run as the build probe. |
| QEMU boot shows Hello World | `qemu-system-x86_64` 8.2.2, raw drive, `-display none`; monitor `xp /40xb 0xb8e60` after 3s | Verified | Bytes `48 65 6c 6c 6f 20 57 6f 72 6c 64 21` ("Hello World!") with attribute `0x0e` (yellow on black). Last row `0xb8f00` is spaces/`0x0e` after `println!` newline. RIP in long mode, `HLT=0` (spinning `loop {}`). This is one environment, not CI. |
| CI on GitHub | Workflow file exists **and** a run is green | Planned | No workflow in tree yet. |
| Second-brain vaults (`research/`) | Paths exist; README explains Procedure / Correction / Relationship / Daily Brief | Verified | Structure present. Not a claim that vaults are richly filled. |
| Thin `ctos-*` roles | `AGENTS.md` + `.cursor/skills/ctos-*/SKILL.md` exist | Verified | Four roles. No `aea-*` names. |
| Integration tests / QEMU test-args | `Cargo.toml` `[package.metadata.bootimage]` test-args + a `#[test_case]` | Planned | Section is commented; args land with M2. |

## How to update

1. Run or cite the probe (command, file path + revision, or `gh run` URL).
2. Change only the rows you probed.
3. If you could not run QEMU, leave boot **Unknown** and say so in the PR. A prior Verified row is one environment; do not copy it forward without a new probe when the boot path changes.
