# Session memory — 2026-09-08 harness + kernel lift

- Remote `main` was LICENSE + stub README only. Canonical kernel lived on the sponsor machine, not GitHub.
- `origin/copilot/rust-os-project` is a host-`std` toy (FIFO scheduler). Not the bare-metal tree. Do not merge it as the kernel.
- Course correction: ctos-native harness vocabulary. Cite architecture.artof.link once as prior art. No `aea-*` roles, no florist/Kafka/AWS, no GitLab SOPs.
- Awkward `println!("Hello World{}", "!")` replaced with `println!("Hello World!");`. Next line is the bare-metal `loop {}`.
- `[package.metadata.bootimage]` left without test-args; comment points at phil-opp testing chapter.
- Honesty: if this cloud box cannot run nightly build-std or QEMU, leave those rows Unknown.
