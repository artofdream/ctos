# Daily brief — 2026-09-12 (Track A / A4 standing EL0 as normal mode)

## Where we stopped

Draft-then-ready PR https://github.com/artofdream/ctos/pull/54 (`cursor/a4-standing-el0-normal-1e14`). Parent is `main` `cf65a6e` (A3 / ADR-023). One PR: ADR-024 standing EL0 as **normal** mode for a loaded image until `exit`, fail-closed restore. No new FR/NFR IDs. App hosting unclaimed. A5–A9 stay **Planned**.

Cloud `scripts/qemu-smoke.sh` **Verified** (2026-09-12, QEMU 8.2.2, `rustc` 1.100.0-nightly `0fc141305`): host `perf: elf-size bytes=4108960`; `el0: task-enter` / `el0: task-active` / `el0: task-exit` / `el0: task-restored` / `el0: restore-fail` / `el0: task-ok`; A1 `svc:*`, A2 `libctos:*`, A3 `loader:*`, and ADR-013 `el0: standing` / `el0: restored` still present; `Running 66 tests` all `[ok]`; force-fail exit 1.

## Do next

1. Human or MRC review. Author does not merge (ADR-002). GitHub author of #54 is expected `cursor[bot]`; merge hat is `artofdream`.
2. A5: isolation completion — remaining identity `.rodata` / `.data` / heap tear. PAN only if `ID_AA64MMFR1_EL1.PAN != 0` on the probe CPU **and** an ADR says so. Do not silent `-cpu` switch.
3. Do not claim “EL0 isolated,” app hosting done, POSIX, or Linux `exec`.

## Honesty

- Cloud Verified is this QEMU virt guest on this revision.
- Standing-as-normal is not isolation. Image is still embedded. No VFS.
