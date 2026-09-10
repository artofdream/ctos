# Session memory — 2026-09-10 (pillar deepen 2–6)

Fetched `origin/main` `1eb82b8` (#18 merged). Branch `cursor/pillar-loops-deepen-48ae`.

Guard store: do **not** resume at LR (abandons the callee frame / retries the `str`). Skip with `ELR+4`. Keep EXPECT_GUARD armed until the caller disarms. First attempt Failed: `guard: fault` then unhandled DABORT `esr=0x96000047` at the same `str`.

EL0 enter: `msr SP_EL0` at EL1 is UNDEF while SPSel=0 (`esr=0x2000000`, same class as the old `MSR SP_EL1` miss). Switch `msr spsel, #1` before writing the user `SP_EL0`, then `ERET` to EL0t. The lower-EL handler already runs SPSel=1 so its `msr sp_el0` (restore kernel SP) is legal.

UXN IABORT on `KERNEL_DATA_BAIT` in `.data` was enough for “cannot execute kernel data.” Shared TTBR0 — do not say isolated.

Host ELF size lives in `qemu-smoke.sh` stdout, not the QEMU serial log.

Do not self-merge. GitHub author of #19 is `cursor[bot]`; merge hat is `artofdream`.
