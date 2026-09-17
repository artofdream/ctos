# ADR-077 — FAT16 nested mkdir + empty rmdir

- Status: Accepted (nested mkdir + empty rmdir mile Verified when the serial / tests pass; Track A / app hosting still Planned for Linux/POSIX claims; Track N / TCP CNTPCT untouched)
- Date: 2026-09-17

## Context

[ADR-073](ADR-073-fat16-mkdir.md) landed FAT16 **root** `mkdir` (`fat: mkdir`). [ADR-075](ADR-075-el0-fs-mkdir.md) added EL0 `fs_mkdir` (27) for root `/edir`. Nested subdirectory trees and empty `rmdir` were explicit follow-ups. [ADR-057](ADR-057-fat16-delete.md) deletes **files** only.

Sponsor follow-up: one-level nested directory create under an existing FAT dir, plus empty `rmdir` fail-closed. Do **not** implement TCP CNTPCT or a new EL0 SVC in this PR (kernel/VFS proof is enough this mile).

## Path grammar (honest)

| Form | Example | Notes |
| --- | --- | --- |
| Flat | `/fdir` | Same as ADR-073: `/` + 1..=8 of `[a-z0-9_-]` |
| One nested | `/fdir/nest` | `/` + parent + `/` + child; each component 1..=8 of `[a-z0-9_-]`; whole path ≤ `PATH_MAX` (16) |
| Deeper | `/a/b/c` | **BadPath** (fail-closed) |
| File ops | `/probe` | create/open/unlink/readdir stay **flat** (`vfs::valid_path`); nested file paths → `BadPath` |

Missing parent on a nested mkdir → `Missing`. Parent name occupied by a **file** → `BadPath`. Name collision → `Exists`.

## Hypotheses

| ID | Option | Notes |
| --- | --- | --- |
| **H1 (chosen)** | Extend `fat::mkdir` / `vfs::mkdir` for one nested level; expose `fat::rmdir` / `vfs::rmdir` for empty dirs. Serial `fat: nested` / `fat: rmdir`. Keep `fat: mkdir`. No new SVC. | Deepens ADR-073; documents grammar. |
| **H2 (rejected)** | Claim POSIX `mkdir` / `rmdir` / `mkdirat` or a Linux directory ABI. | Forbidden honesty. |
| **H3 (rejected)** | New `SYS_FS_RMDIR` / EL0 sample this PR. | Optional; kernel/VFS proof is enough; EL0 can follow. |
| **H4 (rejected)** | Arbitrary-depth trees, LFN, recursive `rm`, multi-cluster directories, exFAT. | Out of scope. |
| **H5 (rejected)** | TCP CNTPCT in this PR. | Separate follow-up; out of scope. |

## Decision

1. **`fat::mkdir`.** Flat root unchanged. Nested `/parent/child`: look up parent dirent, allocate child cluster with `.` / `..` (`..` = parent cluster id), install `ATTR_DIR` dirent in the parent cluster (spc=1). Fail-closed Exists / Missing / BadPath as above.
2. **`fat::rmdir` / `vfs::rmdir`.** Empty only (`.`` / `..` + rest zero). Mark dirent `0xE5`, free cluster chain. Non-empty → `BadHandle`. Missing → `Missing`. File path → `BadPath`. Not recursive rm.
3. **Probe after root mkdir.** Create `/fdir/nest`, confirm dirent + dots, Exists on second create, BadPath on `/fdir/nest/x`, Missing on `/nope/x`. Empty-rmdir nest; refuse rmdir `/fdir` while nest exists; refuse rmdir `/probe`. Serial `fat: nested` then `fat: rmdir`. Keep `fat: mkdir` / grow / delete / net / samples.
4. **No new SVC numbers.** Public ABI stays as today (FS 19–23 + 27, net 24–26 + 28). EL0 `mkdir-libctos` BadPath bait becomes `/a/b/c` (three-level) so Exists/BadPath proof still holds under the new grammar.
5. **Fail-closed smoke.** Grep `fat: nested` / `fat: rmdir`; keep `fat: mkdir` / `fat: ok` / net / samples. `#[test_case]` covers nested+rmdir + path grammar.
6. **Honesty.** Say “the guest created a one-level nested FAT16 directory and removed an empty one through the thin VFS” only when serial / tests pass. Do **not** say: POSIX `mkdir`/`rmdir`, arbitrary directory trees, recursive rm, exFAT/FAT32, “supports FAT,” EL0 isolated, PAN, taken SError, Linux, containers, TCP CNTPCT.
7. **NFR-10 text** revised in place (ID unchanged). Threat-model **v1.52**. Do not mint NFR-15+.

## Consequences

- Code: `src/fat.rs` (parse_dir_path / nested mkdir / rmdir / probe), `src/vfs.rs` (`vfs::rmdir` + mkdir routing), `scripts/qemu-smoke.sh` greps, light `user/mkdir-libctos` BadPath bait update.
- Docs: this ADR, filesystem overview/stance, honesty ledger, roadmap note, threat-model v1.52, SUMMARY, light fr-nfr NFR-10 note.
- Follow-ups (not this PR): EL0 `fs_rmdir` SVC + sample, deeper path trees, multi-cluster directories, recursive rm (non-goal unless sponsored), TCP CNTPCT.
