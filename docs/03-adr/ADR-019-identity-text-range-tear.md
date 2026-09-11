# ADR-019 — High-VA continuation + dedicated identity text *range*

- Status: Accepted (partial tear; live `.text` / `.rodata` / `.data` / heap still identity-mapped)
- Date: 2026-09-11

## Context

[ADR-018](ADR-018-identity-teardown-first-cut.md) cloned TTBR1 RAM tables and unmapped one dedicated identity text page. rustc still emits link-time identity addresses (`relocation-model: static`). `_start` / QEMU `-kernel` still load at `0x4008_0000`.

The preferred next cut was to unmap contiguous live identity `.text` after the boot stub once EL1 fetched from the high alias. A first attempt on this cloud VM **Failed**: after unmapping `[0x4008_1000, __text_end)`, the first `writeln!` / `println!` took an unhandled current-EL sync. `core::fmt::write` takes `&mut dyn Write`; those vtable methods are identity fn pointers. Unmapping live `.text` is therefore blocked until fmt / `dyn` dispatch is proven high-only.

This ADR is the **largest Verified cut** that still boots: jump the post-MMU continuation to its TTBR1 alias, then unmap a **16 KiB dedicated** identity text range (four pages), not one probe page.

## Decision

1. **High-VA continuation.** After `paging::init` (MMU on, high `VBAR_EL1`, ADR-018 first tear page), `kernel_main` `BR`s to the TTBR1 alias of `kernel_main_high`. Serial `ident: jump`. Direct `BL` stays in the high window (PC-relative). UART MMIO stays the absolute identity `0x0900_0000`. Stacks / heap / `.data` stay identity VAs.
2. **Boot stub stays mapped.** The `0x4008_0000` page (`_start` + `exception_vectors`) stays identity-mapped. Live `.text` after that page also stays — rustc fmt / `dyn` still needs those identity fn pointers.
3. **Dedicated 16 KiB tear range.** Linker `__ident_tear_start` … `__ident_tear_end` is four pages: page 0 holds `ident_tear_el1_path` (ADR-018), page 1 holds `ident_range_el1_path`, pages 2–3 are aligned pad. After the high jump, unmap the whole range from kernel and user TTBR0 and `TLBI VAAE1` each low VA. High twins stay PXN-clear.
4. **Keep live `.text` / `.rodata` / `.data` identity-mapped.** Do not yank them here. A botched full `.text` yank is worse than a smaller Verified slice.
5. **Identity `fn` pointers used after the jump go high** where we control them (scheduler trampoline / task entries; custom test runner `dyn Testable` method). That is preparation, not a claim that rustc fmt is high-only.
6. **Fail-closed probe.** Serial `ident: jump`. `ident: range lo=… hi=… pages=N` with `N >= 4`. Existing `ident: split` / `fault` / `high` / `no el0` / `ok` stay. `ident: text` is EL1 fetch of the second torn page via the high twin. `scripts/qemu-smoke.sh` greps those strings and rejects `ident: probe missed` / `ident: leaked` / `ident: range missed`.
7. **Identity boot stub stays.** `_start`, QEMU `-kernel` load, live identity `.text`/`.rodata`/`.data`/heap, PL011 `0x0900_0000`. **Do not say the kernel moved.**
8. **Still Planned.** Unmap live identity `.text` after fmt / `dyn` calls are proven high-only; then `.rodata`/`.data`/heap; PAN on `-cpu cortex-a57`; umbrella EL0 isolation. Do not change default `-cpu`.
9. **NFR-10 text** is revised in place (ID unchanged). Do not mint NFR-15+.

## Honesty

Say “the post-MMU continuation ran at a high VA” or “a 16 KiB dedicated identity text range was unmapped while EL1 still fetched the high twins” only when the serial / tests pass. Do **not** say:

- the kernel has moved to the high half
- identity `.text` after `_start` was fully torn down
- identity mappings were fully torn down
- “secure OS” / “hardened” / “EL0 isolated”
- PAN (still unclaimed on `-cpu cortex-a57`)

## Consequences

- `paging::jump_high` / `tear_identity_text_range` own the cut. `src/teardown.rs` extends the serial probe.
- [ADR-018](ADR-018-identity-teardown-first-cut.md) remains the split-tables + first dedicated page cut.
- [ADR-020](ADR-020-identity-fnptr-reloc.md) rewrites rustc vtables to high aliases and unmaps live identity `.text` after the boot stub. `.rodata` / `.data` / heap stay.
