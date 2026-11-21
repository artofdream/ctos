# 2026-11-21 — Heap payload alignment after a split free-block (FR-10)

- Why: `header_pad` used a fixed offset from `HEADER_SIZE`. Leftover nodes after a split need not be aligned to a later `Layout`, so a 32- or 64-byte allocation could return an under-aligned pointer and break the `GlobalAlloc` contract (FR-10).
- What: Port of the unique `src/heap.rs` fix from `cursor/heap-pointer-alignment-9e96` @ `f8ed690cb56a` onto current `main`. Pad is computed from the free-block address and stashed so dealloc can recover the header. New `#[test_case]` `alloc_honors_align_after_split`.
- Probe: local `./scripts/qemu-smoke.sh` on an amd64 throwaway clone (QEMU 10.2.1 TCG, `rustc` 1.100.0-nightly `0fc141305`): hello serial `heap: ok`; `cargo test` `Running 165 tests` all `[ok]`, including the new case. Scoped to QEMU `cargo test`, not hardware.
- The old branch stays until this PR merges. Review-only; do not self-merge (ADR-002).
