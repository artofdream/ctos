# Session memory — 2026-09-12 (A9 OS/app slots)

Base `main` `f02e783` (A8). Branch `cursor/a9-os-app-slots-cf00`.

Chose H1: two host artifacts + FAT `/hello` + A3 `run_image`. Rejected removing the A2–A4 embed this mile (would break those ratchets). Rejected claiming cross-update from ABI stability.

Storage is the existing A7 raw FAT16 image, not a new format. `/probe` stays. `/hello` is the app ELF (cap 64 KiB, cluster chain).

`perf: app-load` wraps VFS read + parse + map + ERET. Not a bench. Delta vs embed stays Planned.

Do not treat this file as the ledger.
