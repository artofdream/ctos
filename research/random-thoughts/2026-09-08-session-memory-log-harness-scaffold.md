# Session memory — 2026-09-08 harness + kernel lift

- Remote `main` was LICENSE + stub README only. Canonical kernel lived on the sponsor machine, not GitHub.
- `origin/copilot/rust-os-project` is a host-`std` toy (FIFO scheduler). Not the bare-metal tree. Do not merge it as the kernel.
- Course correction: ctos-native harness vocabulary. Cite architecture.artof.link once as prior art. No `aea-*` roles, no florist/Kafka/AWS, no GitLab SOPs.
- Awkward `println!("Hello World{}", "!")` replaced with `println!("Hello World!");`. Next line is the bare-metal `loop {}`.
- `[package.metadata.bootimage]` left without test-args; comment points at phil-opp testing chapter.
- Honesty: first `cargo build` failed (json-target-spec, pointer-width type, rustc-abi, data-layout). Ratcheted into `.cargo/config.toml` and `x86_64-ctos.json`. Then build + bootimage succeeded.
- QEMU probe: last row blank (println newline); row 23 at `0xb8e60` is Hello World! `0x0e`. Do not look only at `0xb8f00`.
- `bootloader` crate is unused in Rust source (bootimage reads it). Warning only; keep the dep.
- Copilot host-std branch remains unrelated.
