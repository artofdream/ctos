# Daily brief — 2026-09-12 (Track A / A5 identity `.rodata` + PAN ADR)

## Where we stopped

Draft-then-ready PR https://github.com/artofdream/ctos/pull/55 (`cursor/a5-identity-tear-pan-0355`). Parent is `main` `e10a613` (A4 / ADR-024). One PR: ADR-025 identity `.rodata` tear + ADR-026 PAN capability (enable Planned). No new FR/NFR IDs. A6–A9 stay **Planned**.

Cloud `scripts/qemu-smoke.sh` **Verified** (2026-09-12, QEMU 8.2.2, `rustc` 1.100.0-nightly `0fc141305`, `-cpu cortex-a57`): host `perf: elf-size bytes=4121360`; `ident: ro-reloc n=589` / `ident: rodata lo=0x400ad000 hi=0x400b8000 pages=11` / `ident: rodata-fault` / `ident: rodata-high`; `pan: id=0` / `pan: absent`; A1–A4 markers still present; `Running 68 tests` all `[ok]`; force-fail exit 1.

GHA on tip `726a4a2` **Verified** (implementer grepped both matrices of PR [34678808063](https://github.com/artofdream/ctos/actions/runs/34678808063)): hello `ident: ro-reloc n=588` / `ident: rodata … pages=11` / `ident: rodata-fault` / `ident: rodata-high` / `pan: id=0` / `pan: absent`; 68 tests; `qemu-smoke: ok`. Push [34678806604](https://github.com/artofdream/ctos/actions/runs/34678806604) success. Bugbot pass. Docs-only ledger follow-up SHAs are not that probe.

## Do next

1. Separate MRC session (`COMMENT` only). Author does not merge (ADR-002). GitHub author of #55 is `artofdream`; merge hat is `cursor[bot]` after this-SHA green + Bugbot resolved-or-declined.
2. A6: thin VFS + memfs ([issue #37](https://github.com/artofdream/ctos/issues/37)). Path walk + read probe. No virtio, no FAT claim.
3. Still Planned: identity `.data` / heap tear (needs SP relocate + high allocator VAs); PAN enable (needs a CPU with `ID_AA64MMFR1_EL1.PAN != 0` — do not silent `-cpu` switch); lower-EL IRQ while standing; EL0 entry without `TLBI VMALLE1`; umbrella isolation.

## Honesty

- Cloud Verified is this QEMU virt guest on this revision. GHA Verified is the grepped `726a4a2` matrices, not a later docs-only SHA.
- Did not claim “EL0 isolated,” PAN enable, or that identity mappings were fully torn down.
