# Session memory — 2026-09-11 (layout L3 / Docker paging miss)

Fetched `origin/main` `b2bbb99` (#20 merged). Branch `cursor/layout-l3-user-map-c915`.

PR #20 hello on this rustc: `__data_start=0x400a6000` `__kernel_end=0x400cd000` (first RAM 2 MiB). Docker ELF `3786384` vs cloud `3785520` vs GHA ~3.69 MiB.

`fill_ram_wx` reused one `L3_RAM`. Block 0 always needs L3 (`KERNEL_TEXT` at `0x40080000`). A second mixed block (data_start past `0x40200000`) overwrites that table. `fill_user_map` required exception stack inside the first 2 MiB.

Guard/ro passing while paging/heap/el0 missed also matches pre-MMU `.bss` stores: `frame::ALLOC` and `USER_MAP_OK` before `SCTLR.C` — same class as the PR #20 boot-delta miss on the larger test image.

Fix: L3 pool; user map walks every overlapping 2 MiB; `frame::init` after `paging::init`; `USER_MAP_OK` after MMU; `dc civac` on the map-window alias. Linker ratchet `. = 0x40201000` for `__data_start`. Did not pin nightly.

Cloud probe printed `paging: layout data=0x40201000 end=0x4023c000 pool=0x4023c000 user=1` then all ok strings.

Do not self-merge. GitHub author of #21 is expected `cursor[bot]`; merge hat is `artofdream`.
