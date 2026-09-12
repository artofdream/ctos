# ADR-020 — High-VA fn-pointer rewrite + live identity `.text` tear

- Status: Accepted (live `.text` after the boot stub; `.rodata` / `.data` / heap still identity-mapped)
- Date: 2026-09-11

## Context

[ADR-019](ADR-019-identity-text-range-tear.md) jumps the post-MMU continuation to its TTBR1 alias and unmaps a 16 KiB dedicated identity text range. A first attempt to unmap live `.text` after `_start` **Failed**: the first `writeln!` / `println!` took an unhandled current-EL sync.

This ADR investigates that failure and takes the largest honest Verified cut toward tearing live identity `.text`.

**Probed root cause (this cloud VM, `main` `24d94e6` hello ELF, then this branch):**

- rustc `dyn Write` vtables live in `.rodata`. The `Pl011` table is `drop=0 / size=8 / align=8 / write_str / write_char / write_fmt` with identity method addresses (`write_char` at `0x4008_1084`, immediately after the vectors page).
- The hello `.rodata` held a handful of other identity `.text` words (`Debug::fmt`, `PadAdapter` as `Write`). `.data` had none. `.text` had no 8-byte identity literal pools.
- AArch64 codegen used `ADRP` (PC-relative), not `MOVZ`/`MOVK` of `0x4008`. After the high-VA jump, new `fn()` values are already high; **tables linked as `R_AARCH64_ABS64` stay identity** until rewritten. The ELF is `ET_EXEC` / `relocation-model: static` — no leftover reloc records to apply.
- `.text` and `.rodata` shared page `0x400a4000` on the ADR-019 layout (vtable at `0x400a4338`). Unmapping “all of `.text`” without a page split would also drop the vtable.

`relocation-model: pic` was considered and **not** taken: static + a post-jump patcher matches the existing `-kernel` / `0x4008_0000` contract and does not need a dynamic linker.

## Decision

1. **Page-split `.text` / `.rodata`.** Linker `__text_end` / `__rodata_start` are 4 KiB aligned so live `.text` and fmt tables do not share a page.
2. **High-VA fn-pointer rewrite.** After `ident: jump`, walk `.rodata` (through `__ident_tear_start`) and `.data`. Every 8-byte word that is an identity address in `.text` or `__ident_tear_*` is rewritten to its TTBR1 alias. RO pages are made writable only for that store, then restored (`SCTLR.WXN` makes a writable page XN — they are not fetched). Serial `ident: reloc n=N` with `N >= 1`. A `Pl011 as dyn Write` fat-pointer read must see a high `write_str`.
3. **Live identity `.text` tear.** Unmap `[0x4008_1000, __text_end)` from kernel and user TTBR0 (`TLBI VAAE1` each low VA). High twins stay PXN-clear. The `0x4008_0000` page (`_start` + `exception_vectors`) stays identity-mapped. Serial `ident: live lo=… hi=… pages=N` with `N >= 8`.
4. **Keep `.rodata` / `.data` / heap identity-mapped.** String literals and rewritten vtables still live there. Do not yank them here.
5. **Keep `_start` / QEMU `-kernel` at `0x4008_0000`.** Do not change default `-cpu`. Do not claim PAN.
6. **Fail-closed probe.** Existing `ident: jump` / `range` / `split` / `fault` / `high` / `text` / `no el0` / `ok` stay. New `ident: reloc` / `ident: live`. `scripts/qemu-smoke.sh` greps those strings and rejects `ident: reloc missed` / `ident: live missed`. `#[test_case]` covers the rewrite and the live unmap.
7. **Still Planned (this ADR).** Unmap identity `.rodata` / `.data` / heap after those accesses are proven high-only; PAN on `-cpu cortex-a57`; umbrella EL0 isolation. [ADR-025](ADR-025-identity-rodata-tear.md) takes the `.rodata` cut. [ADR-026](ADR-026-pan-capability.md) probes the PAN ID field (enable stays Planned).
8. **NFR-10 text** is revised in place (ID unchanged). Do not mint NFR-15+.

## Honesty

Say “rustc vtable / fn-pointer words were rewritten to high aliases” or “live identity `.text` after the boot stub was unmapped while `println!` still ran” only when the serial / tests pass. Do **not** say:

- the kernel has moved to the high half
- identity mappings were fully torn down (`.rodata` / `.data` / heap stay)
- “secure OS” / “hardened” / “EL0 isolated”
- PAN (still unclaimed on `-cpu cortex-a57`)

## Consequences

- `paging::rewrite_identity_fn_ptrs` / `paging::tear_live_identity_text` own the cut. [ADR-019](ADR-019-identity-text-range-tear.md) remains the high-VA jump + dedicated 16 KiB range.
- A later ADR may unmap `.rodata` once string / table accesses are high-only, then `.data` / heap. That work is not this cut.
