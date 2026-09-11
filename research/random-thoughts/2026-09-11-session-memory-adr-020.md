# Session memory — 2026-09-11 (ADR-020 live identity `.text`)

Started from `origin/main` `24d94e6` (#27 merged). Branch `cursor/adr-020-live-text-reloc-322c`.

Investigated the ADR-019 Failed live-`.text` yank. Hello ELF on `24d94e6`:

- `.text` `0x40080000–0x400a4338`, `.rodata` `0x400a4338–0x400ab650` (shared page `0x400a4000` — vtable at `0x400a4338`).
- `Pl011 as Write` vtable: drop=0, size=8, align=8, `write_str=0x40094448`, `write_char=0x40081084`, `write_fmt=0x400810e0`.
- 9 identity `.text` words in `.rodata` (Write + PadAdapter Write + a few Debug::fmt). 0 in `.data`. 0 eight-byte identity pools in `.text`.
- 354 `ADRP`, 0 `MOVZ`/`MOVK` of `#0x4008`. `relocation-model: static` / no leftover relocs.

Did **not** switch to PIC. Post-jump patcher + page-split `.text`/`.rodata`. Rewrite scans `.rodata` through `__ident_tear_start` and `.data`; RO pages temporarily AP-writable (WXN → XN while writable; not fetched). Then unmap `[0x40081000, __text_end)`.

Hello layout after the split: `__text_end=__rodata_start=0x400a6000`, live pages=37, 12 rodata identity-text words (duplicate Write vtable).

PAN unclaimed. `.rodata`/`.data`/heap stay. Do not self-merge.
