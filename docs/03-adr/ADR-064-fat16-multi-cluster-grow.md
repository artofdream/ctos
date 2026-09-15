# ADR-064 — FAT16 multi-cluster grow (write depth)

- Status: Accepted (multi-cluster grow mile Verified when the serial / tests / EL0 markers pass; Track A product claims unchanged; Track N untouched)
- Date: 2026-09-16

## Context

[ADR-050](ADR-050-fat16-write.md) landed guest FAT16 **write** + small create, explicitly **refusing multi-cluster grow** (cap: one cluster / `FILE_MAX` 256). [ADR-057](ADR-057-fat16-delete.md) frees a cluster chain (already walked to EOC). [ADR-061](ADR-061-fat-libctos-sample.md) added an EL0 FAT **read** sample on `/probe`. Reads of multi-cluster host ELFs (`/hello`, …) already walked the FAT chain; **guest write** still stopped at one cluster.

Docs “new vs extend” prefer deepening the known small FS (FAT16) behind the thin VFS. This mile is honest **multi-cluster file grow**: allocate a FAT chain, update dirent size, write across a cluster boundary, prove with kernel serial + unit tests + EL0 chunked SVC write/read-back.

Not exFAT. Not FAT32. Not a POSIX full write API. Not Track N / virtio-net ([ADR-063](ADR-063-network-foundation-scope.md) landed on `main` as docs-only N0 — do not collide; this mile is **064** only).

## Hypotheses

| ID | Option | Notes |
| --- | --- | --- |
| **H1 (chosen)** | Extend `fat::write_at` to allocate additional clusters when `end` crosses a cluster boundary; raise `FILE_MAX` to 1024 (enough for two 512-byte clusters). Kernel probe `/fgrow` (600 bytes). EL0 `fat-libctos` creates `/egrow` and chunked-writes via existing SVC 19–23 (`FS_IO_MAX` 64). Serial `fat: grow` / `libctos: fat-grow`. | Deepens ADR-050. Same path grammar. Fail-closed. |
| **H2 (rejected)** | Claim POSIX `write` / `pwrite` / sparse files / exFAT. | Honesty forbid. |
| **H3 (rejected)** | New SVC numbers or a second write ABI. | Existing `fs_create`/`fs_write`/`fs_read` suffice. |
| **H4 (rejected)** | Only kernel probe; no EL0 proof. | Product-depth train wants EL0 via existing SVCs. |
| **H5 (rejected)** | Touch Track N / virtio-net. | Separate ladder; out of scope. |

## Decision

1. **`FILE_MAX` = 1024.** Shared memfs/FAT cap. Was 256 (below one 512-byte cluster); multi-cluster grow is impossible under that ceiling. Still small; not a product volume claim.
2. **`extend_chain` / multi-cluster `write_at`.** Walk the existing chain; allocate free clusters, link prior→new→EOC, zero new clusters; write payload across cluster boundaries. Refuse `end > FILE_MAX` and empty writes (`TooBig`). No sparse holes (`off > size` → `BadHandle`). Still assumes `spc=1` (one sector per cluster) this mile.
3. **Kernel probe `/fgrow`.** After delete probe: create, write 600 patterned bytes, confirm FAT `chain_len ≥ 2`, VFS read-back, boundary bytes differ. Serial `fat: grow`. Keep `fat: ok` / write / readdir / delete / A9.
4. **EL0 via existing SVCs.** Extend `user/fat-libctos`: after `/probe` read, create-or-open `/egrow`, write 600 bytes in 64-byte SVC chunks, read-back boundary bytes with immediates. Serial `libctos: fat-grow` then `libctos: fat-ok`. Fail → `libctos: fat-fail`.
5. **Fail-closed smoke + tests.** Grep `fat: grow` / `libctos: fat-grow`; reject missing. `#[test_case]` `fat16_multi_cluster_grow` + over-`FILE_MAX` refuse.
6. **Honesty.** Say “the guest grew a FAT16 file across a cluster boundary through the thin VFS” only when markers pass. Do **not** say: POSIX write API, exFAT/FAT32, “supports FAT,” app hosting reopen, EL0 isolated, PAN, taken SError, networking.
7. **NFR-10 text** revised in place (ID unchanged). Threat-model **v1.39** (slice after #109 / ADR-063 threat-model v1.38). Do not mint NFR-15+.

## Consequences

- Code: `src/fat.rs` (extend_chain / write_at / grow probe), `src/vfs.rs` (`FILE_MAX`), `user/fat-libctos/`, `scripts/qemu-smoke.sh`.
- Docs: this ADR, filesystem overview/stance, honesty ledger, SUMMARY, threat-model v1.39, light fr-nfr NFR-10 note.
- Follow-ups (not this PR): `spc>1`, shrink-on-truncate, exFAT, POSIX write API claim, Track N N1.
