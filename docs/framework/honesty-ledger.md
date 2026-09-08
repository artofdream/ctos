# ctos honesty ledger

Status words are claims. Each row needs a **probe**. Unprobed = **Unknown**. Never round Unknown up to Verified because the file exists, a PR is open, or a README teaches `cargo run`.

Allowed flags: **Verified** (probe passed), **Unknown** (no probe or probe blocked), **Planned** (not built yet), **Failed** (probe ran and lost).

| Claim | Probe | Status | Notes |
| --- | --- | --- | --- |
| VGA hello source present (`println!("Hello World!");` + `vga_buffer` writer) | Read `src/main.rs` and `src/vga_buffer.rs` on this branch | Verified | Source inspection only. Not a boot. |
| `.cargo/config.toml` present (build-std, `x86_64-ctos.json`, bootimage runner) | Read `.cargo/config.toml` | Verified | Presence only. Does not prove `cargo build` succeeds. |
| `cargo build` for `x86_64-ctos.json` | Run `cargo build` with nightly + rust-src in this environment | Unknown | Not probed at ledger write time; update if a later probe lands. |
| QEMU boot shows Hello World | `cargo run` / QEMU window or serial capture | Unknown | Do not treat README instructions as a boot. |
| CI on GitHub | Workflow file exists **and** a run is green | Planned | No workflow in tree yet. |
| Second-brain vaults (`research/`) | Paths exist; README explains Procedure / Correction / Relationship / Daily Brief | Verified | Structure present. Not a claim that vaults are richly filled. |
| Thin `ctos-*` roles | `AGENTS.md` + `.cursor/skills/ctos-*/SKILL.md` exist | Verified | Four roles. No `aea-*` names. |
| Integration tests / QEMU test-args | `Cargo.toml` `[package.metadata.bootimage]` test-args + a `#[test_case]` | Planned | Section is commented; args land with M2. |

## How to update

1. Run or cite the probe (command, file path + revision, or `gh run` URL).
2. Change only the rows you probed.
3. If you could not run QEMU, leave boot **Unknown** and say so in the PR.
