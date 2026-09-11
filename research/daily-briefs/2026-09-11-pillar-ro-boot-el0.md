# Daily brief — 2026-09-11 (RO+NX / boot-delta / user TTBR0)

## Where we stopped

Draft PR on `cursor/pillar-deepen-ro-boot-el0-aadc`. Parent is `main` `72c8da0` (Merge PR #19). One PR: ADR-015 RO+NX, NFR-08 boot-delta, ADR-013 user-TTBR0 read mile. No new FR/NFR IDs.

Cloud `scripts/qemu-smoke.sh` **Verified** (2026-09-11, QEMU 8.2.2, `rustc` 1.100.0-nightly `67eda617e`): host `perf: elf-size bytes=3785520`; `ro: nx data` / `ro: write fault` / `ro: ok`, `el0: svc` / `el0: nx kernel` / `el0: no kernel read` / `el0: ok`, `perf: boot-delta ticks=111019`, `perf: cntpct delta=20506`, `perf: irq-delta min=4885 max=26301 spread=21416 n=8`, `input: rx 0x41`, two `exception: sync BRK`, `exception: fatal nested` / `kind=0x200`. `Running 36 tests` all `[ok]`, force-fail exit 1.

First `cargo test` Failed: pre-MMU boot-delta mark lost on the test image; early sample moved to after `paging::init`.

GHA `smoke.yml` on this branch: **Unknown**.

## Do next

1. Run `scripts/qemu-smoke.sh` and paste the real snippet into the honesty ledger.
2. Human or MRC review. Author does not merge (ADR-002).
3. Isolation (PAN / ASID TLB / standing EL0 / TTBR1) stays Planned. Do not claim “EL0 isolated.”
4. Identity image is W^X on this virt guest only. Do not say “secure OS.”

## Honesty

- Isolation row stays Planned. User TTBR0 + cannot-read is a different mile.
- PAN unclaimed (`cortex-a57` / ARMv8.0).
- Boot-delta is a measurement, not a budget.
- cts-ai Docker Desktop path not re-run.
