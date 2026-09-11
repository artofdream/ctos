# Session memory — 2026-09-11 (TTBR1 high-VA exec)

Fetched `origin/main` `8745939` (#24 merged). Branch `cursor/ttbr1-high-el1-exec-136c`. Draft PR #25.

ADR-017: `L1_HIGH[1] → L2_RAM` aliases identity RAM at `va + 0xFFFF_FF80_0000_0000`. `ttbr1_high_el1_path` invoked via high fn pointer; prints `ttbr1: el1 exec`. `VBAR_EL1` relocated after MMU (`ttbr1: vbar`). Shared tables — unmap identity unmaps the alias.

Failed then fixed:

1. Guard store: high handler `addr_of!(__stack_guard)` vs identity `FAR_EL1` → unhandled `esr=0x96000047`. Fix: `linker_sym` masks to 39-bit identity.
2. Standing SVC #1 then hang: `stay_at_el0` programmed high `L1_USER` as TTBR0. Fix: `identity_pa` on all table PAs.

Fatal nested ELR on this probe was `0xffffff80400839e4` (handler at high VA). Hello `kernel_main` BRK ELRs stayed identity.

PAN unclaimed. Identity teardown Planned.
