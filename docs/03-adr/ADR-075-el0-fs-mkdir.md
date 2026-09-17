# ADR-075 — EL0 `fs_mkdir` SVC + freestanding `mkdir-libctos` sample

- Status: Accepted (EL0 mkdir mile Verified when the serial / tests pass; POSIX `mkdir` / nested trees / Track N EL0 TCP still non-goals)
- Date: 2026-09-17

## Context

[ADR-073](ADR-073-fat16-mkdir.md) landed kernel `fat::mkdir` / `vfs::mkdir` with serial `fat: mkdir` and **deferred** the public EL0 SVC + sample. Public ABI today: FS SVCs 19–23, net 24–26 ([syscall.md](../framework/syscall.md)). Catalog samples exist for hello / VFS / FAT / yield / net / udp.

Sponsor follow-up unlocks EL0 directory create through the thin VFS — same fail-closed Exists / BadPath honesty as ADR-073. Do **not** implement EL0 TCP or nested directory trees in this PR.

FAT 8.3 path grammar caps the sample slot at eight characters. `/mkdirdemo` (9) is rejected by `path_to_83`; the slot is **`/mkdemo`**.

## Hypotheses

| ID | Option | Notes |
| --- | --- | --- |
| **H1 (chosen)** | Add `SYS_FS_MKDIR` (27). Path ABI matches create/open (`x0` ptr, `x1` len). Map `Ok`→0, `Exists`→1, `BadPath`→2, other→`u64::MAX`. New crate `user/mkdir-libctos` on FAT `/mkdemo`. Print `libctos: mkdir-hi` / `libctos: mkdir-ok`. Kernel `mkdemo: ok`. | Minimal ABI; proves EL0 mkdir + Exists/BadPath without nested trees. |
| **H2 (rejected)** | Claim POSIX `mkdir` / `mkdirat` or Linux mkdir ABI. | Forbidden honesty. |
| **H3 (rejected)** | Extend `fat-libctos` instead of a seventh catalog slot. | Prior miles used a dedicated sample per capability; keep that style. |
| **H4 (rejected)** | Nested subdirectory trees / richer path grammar / recursive create. | Out of scope; no nested claim unless proven (not this mile). |
| **H5 (rejected)** | EL0 TCP SVC in this PR. | Separate follow-up; out of scope. |

## Decision

1. **`SYS_FS_MKDIR` = 27.** Document in [syscall.md](../framework/syscall.md). Args: `x0` = user path pointer, `x1` = length. Kernel copies the path and calls `vfs::mkdir`. Return:
   - `0` (`FS_MKDIR_OK`) on success
   - `1` (`FS_MKDIR_EXISTS`) on `FsError::Exists`
   - `2` (`FS_MKDIR_BAD_PATH`) on `FsError::BadPath` (including failed user-path copy / invalid UTF-8 path)
   - `u64::MAX` (`FS_ERR`) for other failures (Missing mount, Full, …)
   Unknown SVCs still park. Not POSIX. Not a process ABI.
2. **`libctos`.** `ctos_fs_mkdir` / `fs_mkdir` + `SYS_FS_MKDIR` / `FS_MKDIR_*` constants. CRT must not issue reserved 0–2.
3. **Freestanding sample.** `user/mkdir-libctos` on FAT `/mkdemo` (8.3 `MKDEMO`). Creates `/edir` (separate from kernel `/fdir`), requires second create → Exists, nested `/bad/nest` → BadPath. Markers: `libctos: mkdir-hi` / `libctos: mkdir-ok` / `mkdemo: ok`. Keep prior six samples + `fat: mkdir` + all net greps.
4. **Fail-closed smoke.** Grep the new markers; keep `fat: mkdir` / `fat: ok` / grow / delete / net / samples. `#[test_case]` on the slot module.
5. **Honesty.** Say “EL0 created a FAT16 root directory through `fs_mkdir`” only when serial / tests pass. Do **not** say: POSIX `mkdir`, directory trees, exFAT/FAT32, “supports FAT,” EL0 isolated, PAN, taken SError, Linux, containers, EL0 TCP.
6. **NFR-10 text** revised in place (ID unchanged). Threat-model **v1.50**. Do not mint NFR-15+.

## Consequences

- Code: `src/syscall.rs`, `libctos/`, `user/mkdir-libctos/`, `src/mkdemo.rs`, `build.rs`, `scripts/mkfat16.py`, `scripts/qemu-smoke.sh`.
- Docs: this ADR, syscall.md, honesty ledger, filesystem overview/stance, what-can-run, SUMMARY, roadmap note, threat-model v1.50, light fr-nfr NFR-10 note, `user/README.md`.
- Follow-ups (not this PR): nested dirs / richer path grammar, empty `rmdir` as a public mile, EL0 TCP SVC (separate ADR).
