# Session memory — 2026-09-11 (RO+NX / boot-delta / user TTBR0)

Fetched `origin/main` `72c8da0` (#19 merged). Branch `cursor/pillar-deepen-ro-boot-el0-aadc`.

RO+NX: page-align `__data_start`. First RAM 2 MiB stays L3. Text AP[2]=1; data/stacks/heap PXN. `SCTLR.WXN` on once text is RO. Execute-from-`.data` resumes at LR (same as heap NX). Write-to-RO-text skips `ELR+4` (same as guard — do not resume at LR).

User TTBR0: `L1_USER` maps text/rodata + exception stack + shared map-window L2. Omits `.data`. Kernel L1 PA parked in `TPIDR_EL1`. `sync_lower_el` restores TTBR0 before any `.bss` access (`EL0_KSP` lives there). `ldr x0, =L1` would have needed a no_mangle; TPIDR avoids that.

EL0 fetch of unmapped `.data` is now a **translation** IABORT, not only UXN permission. Handler accepts both.

PAN: do not `msr pan` on cortex-a57. Read `ID_AA64MMFR1_EL1` only.

Boot-delta: sample after BSS in `kernel_main`, not in `_start` (BSS wipe). Ready = after `Hello World!`.

Do not self-merge. GitHub author of this PR is expected `cursor[bot]`; merge hat is `artofdream`.
