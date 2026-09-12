# Identity `.data` tear (ADR-037)

Date: 2026-09-12. Machine notes: (evo-x2) intent; implementation verified via Docker/GHA when probes pass.

After ADR-025, `.data`/stacks/heap stayed identity-mapped. This mile:

1. Rewrites ABS64 words pointing into `[__data_start, __kernel_end)` (`ident: data-reloc`) before the `.rodata` unmap so identity `.rodata` is still walkable.
2. Relocates SP / SP_EL1 to the TTBR1 twin of the same physical stack pages.
3. Switches page-table CPU accessors to the high twin (tables live in `.bss`).
4. Unmaps identity `.data`/`.bss`/linker stacks (`ident: data` / `ident: data-fault` / `ident: data-high`).
5. Leaves heap identity-mapped (`ident: heap-stay`).

Not “the kernel moved.” Not “EL0 isolated.” PAN enable stays Planned on `-cpu cortex-a57`.
