# ADR-019 — Identity `.text` range tear (high-VA jump + boot stub stays)

- Status: Accepted (partial tear; `.rodata` / `.data` / heap still identity-mapped)
- Date: 2026-09-11

## Context

[ADR-018](ADR-018-identity-teardown-first-cut.md) cloned TTBR1 RAM tables and unmapped one dedicated identity text page. rustc still emits link-time identity addresses (`relocation-model: static`). `_start` / QEMU `-kernel` still load at `0x4008_0000`. A complete unmap of `.rodata` / `.data` / heap after rewriting every pointer is still too large for one honest PR.

This ADR is the **largest Verified cut** that still boots on virt: after MMU + high VBAR, jump the post-MMU continuation to its TTBR1 alias, then unmap the contiguous identity **`.text`** range after the documented boot stub. Those pages stay executable via the high alias.

## Decision

1. **High-VA continuation.** After `paging::init` (MMU on, high `VBAR_EL1`, ADR-018 tear page), `kernel_main` `BR`s to the TTBR1 alias of `kernel_main_high`. Direct `BL` stays in the high window (PC-relative). UART MMIO stays the absolute identity `0x0900_0000`. Stacks / heap / `.data` stay identity VAs.
2. **Boot stub stays mapped.** The `0x4008_0000` page (`_start` + `exception_vectors`) stays identity-mapped until a later jump-complete of that stub. `VBAR_EL1` already fetches the high twin.
3. **Tear identity `.text` after the stub.** Linker `__text_end` is page-aligned before `.rodata`. Unmap `[0x4008_1000, __text_end)` from kernel and user TTBR0 and `TLBI VAAE1` each low VA. High twins stay PXN-clear. The ADR-018 `__ident_tear_*` page stays torn.
4. **Keep `.rodata` identity-mapped.** rustc stores absolute string / vtable / `fn` pointers in `.rodata`. Unmapping those leaves would fault `println!` and `dyn` dispatch even when PC is high. `.data` / heap / linker stacks / UART MMIO stay. Do not yank them here.
5. **Identity `fn` pointers go high.** Scheduler trampoline / task entries and the custom test runner `dyn Testable` method are invoked through `to_high_va`. A raw identity `BLR` into torn `.text` is a current-EL translation IABORT.
6. **Fail-closed probe.** Serial `ident: jump` (high continuation). `ident: range lo=… hi=… pages=N` with `N >= 2`. Existing `ident: split` / `fault` / `high` / `no el0` / `ok` stay. `ident: text` is EL1 fetch of a torn ordinary `.text` page via the high twin. `scripts/qemu-smoke.sh` greps those strings and rejects `ident: probe missed` / `ident: leaked` / `ident: range missed`.
7. **Identity boot stub stays.** `_start`, QEMU `-kernel` load, remaining identity `.rodata`/`.data`/heap, PL011 `0x0900_0000`. **Do not say the kernel moved.**
8. **Still Planned.** Unmap identity `.rodata`/`.data`/heap after those accesses are proven high-only; PAN on `-cpu cortex-a57`; umbrella EL0 isolation. Do not change default `-cpu`.
9. **NFR-10 text** is revised in place (ID unchanged). Do not mint NFR-15+.

## Honesty

Say “identity `.text` after the boot stub was unmapped while EL1 still fetched the high twins” only when the serial / tests pass. Do **not** say:

- the kernel has moved to the high half
- identity mappings were fully torn down
- “secure OS” / “hardened” / “EL0 isolated”
- PAN (still unclaimed on `-cpu cortex-a57`)

## Consequences

- `paging::jump_high` / `tear_identity_text_range` own the cut. `src/teardown.rs` extends the serial probe.
- [ADR-018](ADR-018-identity-teardown-first-cut.md) remains the split-tables + one dedicated page cut.
- A later ADR may unmap identity `.rodata`/`.data`/heap. That work is not this cut.
