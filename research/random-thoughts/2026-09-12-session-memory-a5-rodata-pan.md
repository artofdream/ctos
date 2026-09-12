# Session memory — 2026-09-12 (Track A / A5)

Fetched `origin/main` `e10a613` (A4 merge). Branch `cursor/a5-identity-tear-pan-0355`. PR #55.

A5 is not VFS. ADR-025: after live `.text` tear, rewrite ABS64 pointers *to* `.rodata`, then unmap `[__rodata_start, __ident_tear_start)` from kernel + user TTBR0. High twin stays. `ident: ro-reloc n=589` (hello) / `n=1038` (test image). 11 identity `.rodata` pages torn on hello. EL1 identity LDR → `ident: rodata-fault`; high LDR of `IDENT_RODATA_MAGIC` → `ident: rodata-high`. `println!` after the tear still runs.

`.data` / heap / linker stacks stay. SP is still identity `__stack_top`. `GlobalAlloc` still returns identity frame VAs. Those are the next identity cuts, not this mile.

ADR-026: `mrs ID_AA64MMFR1_EL1`, PAN field bits [23:20] = 0 on `-cpu cortex-a57`. Serial `pan: id=0` / `pan: absent`. No `MSR PAN`. Scripts still `-cpu cortex-a57`. Enable + EL1-vs-EL0 fault stay Planned.

68 tests. qemu-smoke ok. Lower-EL IRQ and EL0-without-`TLBI VMALLE1` still Planned.

GHA on `726a4a2`: PR [34678808063](https://github.com/artofdream/ctos/actions/runs/34678808063) both jobs grepped (hello `ident: ro-reloc n=588` / `pan: absent` / 68 tests / `qemu-smoke: ok`). Push [34678806604](https://github.com/artofdream/ctos/actions/runs/34678806604) success. Bugbot pass. Ledger CI row flipped Unknown → Verified on that SHA only.

Do not self-merge. GitHub author of #55 is `artofdream`. Merger is `cursor[bot]`.
