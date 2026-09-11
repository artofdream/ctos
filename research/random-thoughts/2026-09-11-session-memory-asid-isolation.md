# Session memory — 2026-09-11 (ASID isolation)

Fetched `origin/main` `7dbd7e6` (#22 merged; #21 `71ee15f` already on main). Branch `cursor/el0-asid-isolation-f98d`.

ASID isolation: two EL1 tables. Kernel L1 + ASID 1 vs clone L1 (shared MMIO + `L2_RAM`, own map-window L3) + ASID 2. Probe VAs at `MAP_WINDOW+5*4KiB` (dual PA) and `+6*4KiB` (ASID-1 only). Leaves have `nG`. Switch is `msr ttbr0_el1` + `isb` — no `tlbi vmalle1`. Stale ASID-1 magic under ASID 2 prints `asid: stale` and fails the smoke.

Did not flip `is_active()`. Standing EL0 deferred: trampoline flag ≠ user task; lower-EL IRQ still parked.

EL0 `eret_to_el0` / `sync_lower_el` still `TLBI VMALLE1`. Kernel `.data` is global; dropping that flush would leak a cached `.data` translation into the user ASID.

Cloud probe printed `asid: dual` then `asid: conflict` then `asid: ok`. 39 tests.

Do not self-merge. GitHub author of #23 is expected `cursor[bot]`; merge hat is `artofdream`.
