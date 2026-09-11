# Daily brief — 2026-09-11 (layout L3 / Docker paging miss)

## Where we stopped

Draft PR https://github.com/artofdream/ctos/pull/21 (`cursor/layout-l3-user-map-c915`). Parent is `main` `b2bbb99` (Merge PR #20). One PR: kernel L3 pool, multi-block user TTBR0, post-MMU `frame::init` / `USER_MAP_OK`, linker `__data_start=0x40201000`. No new FR/NFR IDs.

cts-ai Docker on `b2bbb99`: **Failed** (sponsor serial, `elf-size bytes=3786384`): `paging`/`heap`/`sched`/`wx`/`el0` probe missed; guard/ro still ok.

Cloud `scripts/qemu-smoke.sh` **Verified** (2026-09-11, QEMU 8.2.2, `rustc` 1.100.0-nightly `67eda617e`): host `perf: elf-size bytes=3837736`; `paging: layout data=0x40201000 end=0x4023c000 pool=0x4023c000 user=1`; all hello probes ok; `Running 38 tests` all `[ok]`; force-fail exit 1.

GHA `smoke.yml` on this branch: **Unknown** (no run URL on the cloud-probe commit yet). Docker after the fix: **Unknown** until the sponsor re-runs `./scripts/docker-smoke.sh` after merge.

## Do next

1. Human or MRC review. Author does not merge (ADR-002). GitHub author of #21 is expected `cursor[bot]`; merge hat is `artofdream`.
2. Sponsor re-probes cts-ai Docker Desktop after merge. This cloud VM has no Docker engine.
3. Isolation (PAN / ASID TLB / standing EL0 / TTBR1) stays Planned. Do not claim “EL0 isolated.”
4. Identity image is W^X on this virt guest only. Do not say “secure OS.”

## Honesty

- Docker Failed row is the sponsor serial on `b2bbb99`, not this VM.
- Cloud Verified is this QEMU virt guest on the dual-L3 ratchet layout.
- Did not pin an old nightly.
