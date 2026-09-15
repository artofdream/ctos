# ADR-062 — Fourth freestanding sample (`yield-libctos`) — cooperative yield rounds

- Status: Accepted (sample + smoke Verified when serial / tests pass; product freestanding app hosting remains Verified under ADR-048/052 — do not reopen)
- Date: 2026-09-16

## Context

[ADR-022](ADR-022-libctos-crt.md) / hello prints `libctos: hi`, yields **once**, then `libctos: ok`. [ADR-059](ADR-059-fs-libctos-sample.md) and [ADR-061](ADR-061-fat-libctos-sample.md) deepened the catalog on VFS/FAT. Product gap remains: no freestanding sample that exercises **several cooperative `yield_now()` rounds** from standing EL0 (still single-task; not preemption).

This mile adds a **fourth freestanding EL0 sample** that yields across multiple uart beats, publishes it on FAT beside `/hello` / `/fsdemo` / `/fatdemo`, expands the catalog, and fail-closes smoke on new markers. Honesty: not POSIX, not Linux, not “EL0 isolated,” not preemption, not a process table.

## Hypotheses

| ID | Option | Notes |
| --- | --- | --- |
| **H1 (chosen)** | New crate `user/yield-libctos`. Print `libctos: yld-hi`, then several `libctos: beat` across `yield_now()` rounds, then `libctos: yld-ok`. FAT slot `/yldemo` (8.3 `YLDEMO`). Kernel loads after ADR-061 fatdemo. Serial greps + `yldemo: ok`. ADR + catalog docs. | Cleanest distinct fourth product; keeps prior three intact. |
| **H2 (rejected)** | Teach `hello-libctos` to loop yields. | Collapses A2 markers; muddies hello smoke greps. |
| **H3 (rejected)** | Preemptive timer slice / multi-task EL0 / process table this mile. | Out of honesty scope; cooperative only. |
| **H4 (rejected)** | Replace `/hello` or another slot. | Breaks A9 / ADR-059 / ADR-061 cross-update and markers. |
| **H5 (rejected)** | Claim POSIX / Linux / “EL0 isolated” / preemption. | Honesty forbid. |

## Decision

1. **Sample crate.** `user/yield-libctos`: same `aarch64-ctos-user` / `linker.ld` / `build-std` as hello/fs/fat. `main` prints `libctos: yld-hi`, then ≥3 `libctos: beat` lines each after `yield_now()`, then one more yield and `libctos: yld-ok`.
2. **Publish.** `build.rs` builds and publishes `target/yield-libctos.elf`. `scripts/mkfat16.py --app4` writes 8.3 `YLDEMO`. Keep `/hello` + `/fsdemo` + `/fatdemo` + their `*: ok` markers.
3. **Kernel probe.** `src/yldemo.rs` loads FAT `/yldemo`, `ERET`s via `loader::run_image_expecting` (last uart len 16 for `libctos: yld-ok\n`), and requires `syscall::yield_count() >= 3`. Serial `yldemo: fat` / `yldemo: mapped` / `yldemo: ok`. Runs after the ADR-061 fatdemo probe.
4. **Smoke.** Fail-closed greps for the new markers (≥3 `libctos: beat`); preserve all existing greps.
5. **Docs.** This ADR; honesty ledger evidence-only; `user/README.md`; [what-can-run.md](../overview/what-can-run.md); [apps-today.md](../framework/apps-today.md); SUMMARY. Threat-model **v1.37** (NFR-10 text in place).
6. **Honesty.** Do **not** claim: EL0 isolated, PAN enable, taken SError, Linux/POSIX/containers, preemption, multi-task EL0, process table, immutable-OS marketing, or reopen “app hosting is done.” Never yank `_start`. No silent `-cpu`.

## Consequences

- Code: `user/yield-libctos/`, `build.rs`, `scripts/mkfat16.py`, `scripts/qemu-aarch64.sh`, `scripts/qemu-smoke.sh`, `src/yldemo.rs`, `src/main.rs`.
- Docs: this ADR + catalog / ledger / threat-model v1.37.
- Follow-ups (not this PR): preemptive scheduling (separate ADR), multi-task EL0, EL0 FAT create/write sample.
