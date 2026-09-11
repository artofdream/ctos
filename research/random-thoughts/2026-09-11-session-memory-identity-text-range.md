# Session memory — 2026-09-11 (ADR-019 identity text range)

Fetched `origin/main` `b0f0ee5` (#26 merged). Branch `cursor/identity-tear-text-range-ea15`. Draft PR #27.

Tried to jump `kernel_main` high and unmap `[0x40081000, __text_end)`. Serial: `ident: jump` then `exception: unhandled sync` then a fatal-nested flood. Cause: `core::fmt::write` takes `&mut dyn Write`; vtable methods are identity addresses. First `writeln!` after tearing live `.text` IABORTs.

Fell back to: keep the high-VA jump (`ident: jump`); grow `__ident_tear_*` to 16 KiB (page 0 `ident_tear_el1_path`, page 1 `ident_range_el1_path`, two pad pages); unmap that range only. `tear_identity_probe_page` still assumed `end == start+4K` and left `IDENTITY_TEAR_OK` false until that check was relaxed.

`ttbr1_high_el1_path as usize` after the jump was a high VA (ADRP); compare to `data_start` failed until `identity_pa`. Hello `observe_probe` may still run at identity (rustc BLR); do not require `pc_is_high()` there.

Hello BRK ELRs are now high (`0xffffff8040082758`). 48 tests. Scheduler trampoline / test-runner dyn method go through `to_high_va`.

GHA on `ac3ad0b` Failed both matrices: `ident: range` then `ident: probe missed` (init still required `end == start+4K`). This SHA relaxes that and publishes `.bss` flags via identity PA.

PAN unclaimed. Full teardown Planned. Do not self-merge. GitHub author of #27 is `artofdream`; merge hat is `cursor[bot]` after this-run green checks (ADR-002). This session does not merge.
