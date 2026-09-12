# Session memory — 2026-09-12 (A3 guest ELF loader)

Fetched `origin/main` `61b2640` (A2 merged). Branch `cursor/a3-elf-loader-ttbr0-49f0`. Draft PR #53.

Chose H1 (ELF64 LE AArch64 `ET_EXEC`, `PT_LOAD` only) over H2 (raw flatten — A2-shaped) and H3 (Linux ABI / `PT_INTERP`).

Hello ELF has three `PT_LOAD`s: R headers at `0x80000000`, RX `.text` at `0x80002000`, R `.rodata` at `0x80002054` (same page as text). Loader unions Ro+Exec on one page; rejects W+X including two segments that share a page.

A2 memcpy path kept. A3 embeds the raw ELF, maps fresh frames, `ERET`s to `e_entry`. Stack at `LOADER_STACK_VA` (`0x80007000`).

First `qemu-smoke` hello printed `loader: ok` then the script died: this session edited `qemu-smoke.sh` while it was running (`dash` syntax error). Re-run after the edit was idle: hello greps passed; `cargo test` failed (`Image` lacked `PartialEq`; W+X overlap fixture used `p_offset=0x100` past a 192-byte buffer → `BadLoad`). Fixes: derive `PartialEq`; offset `0x80`. Then `qemu-smoke: ok` (64 tests + force-fail).

Do not self-merge. A4 is standing-as-normal.
