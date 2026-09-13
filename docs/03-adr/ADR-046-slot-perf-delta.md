# ADR-046 — A9 slot-disconnect performance delta (probe-only embed)

- Status: Accepted (compared CNTPCT pair on one boot — QEMU TCG lab measurement, not a published bench).
- Date: 2026-09-13

## Context

[A9 / ADR-030](ADR-030-os-app-slots.md) prints `perf: app-load ticks=<n>` around the FAT `/hello` read + A3 map + `ERET`. [ADR-032](ADR-032-track-a-leftovers.md) removed the production A2–A4 / A9 `include_bytes!` path (FAT-only; smoke rejects `slot: embed`). The honesty ledger still had a **Planned** row: compare that FAT trip to the old embedded A3 trip on the **same guest**.

Do **not** restore production embed. Do **not** invent a percent or “faster/slower” claim. Do **not** switch `-cpu`. Keep `_start` at `0x4008_0000`. QEMU TCG jitter is one lab — not criterion, not SPEC.

## Hypotheses

| ID | Option | Notes |
| --- | --- | --- |
| **H1 (chosen)** | Probe-only `include_bytes!` of the same `OUT_DIR/hello-libctos.elf` in `src/slot.rs`. Time map+`ERET` with CNTPCT → `perf: embed-load ticks=<n>`. Keep FAT `perf: app-load`. Print `perf: slot-delta app=<a> embed=<b>` (raw ticks). Never print `slot: embed`. | Honest compared pair. Production slot stays FAT-only. |
| **H2 (rejected)** | Restore A3 embed as a production fallback when FAT is missing. | Violates ADR-032. Smoke must keep rejecting `slot: embed`. |
| **H3 (rejected)** | Close the Planned row as **non-comparable** (embed removed) without measuring. | Prefer H1 when feasible. Docs-only fallback only if dual-path is unsafe. |
| **H4 (rejected)** | Publish a percent / “slots are free” / criterion bench. | NFR-07 forbids invented benches. |

## Decision

1. **Production path unchanged.** A9 still loads FAT `/hello` only. Serial still prints `slot: fat` / `slot: mapped` / `slot: ok` / `perf: app-load ticks=<n>`. Missing `/hello` is still `slot: probe missed`. `slot: embed` must never appear.
2. **Probe-only embed.** `src/slot.rs` may `include_bytes!(concat!(env!("OUT_DIR"), "/hello-libctos.elf"))` for measurement. Bytes must match FAT `/hello` / `HELLO_ELF_LEN`. A2–A4 (`src/libctos.rs`, `src/loader.rs`) must **not** embed.
3. **Markers.** After a successful FAT trip on the same boot: `perf: embed-load ticks=<n>` then `perf: slot-delta app=<a> embed=<b>`. Fail closed on `perf: embed-load missed`. No ratio percent; no “faster/slower” words on the serial line.
4. **Smoke.** `scripts/qemu-smoke.sh` greps the new markers, still rejects `slot: embed`, rejects A2/A3 hello `include_bytes!`, requires the slot.rs probe include, and rejects `slot-delta` lines that claim percent/faster/slower.
5. **Honesty.** This is a **measurement pair** on QEMU `virt` TCG. It is not a latency budget, not a product KPI, and not “slots are free.” Record Verified only with serial evidence on a named tip.

## Consequences

- Code: `src/slot.rs` (probe include + markers); smoke greps; `#[test_case]` that embed CNTPCT advanced and bytes match FAT.
- Docs: [performance.md](../framework/performance.md), [measure.md](../overview/measure.md), honesty ledger, track-a / roadmap as needed.
- Kernel debug ELF grows by the hello ELF size (~8 KiB). That is expected for the probe.
- Isolation leftovers (PAN enable, taken SError, yank `_start`, umbrella) stay Planned — unchanged by this mile.
