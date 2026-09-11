# Session memory — 2026-09-11 (standing EL0 + TTBR1)

Fetched `origin/main` `fb050b7` (#23 merged). Branch `cursor/el0-standing-ttbr1-5ce0`. Draft PR #24.

Standing EL0: user page `SVC #1` / `MOVZ X1, #0x51A4` / `SVC #2` on user TTBR0. `is_active()` true only between `install_standing` and `clear_active`. First SVC stays at EL0 (restore user TTBR0 in the handler; AArch64 SVC preferred return is already the next insn — do **not** add 4). First attempt Failed: `stay_at_el0` added 4, skipped MOVZ, SVC #2 `esr=0x56000002 elr=0x8000200c`, `x1` ≠ magic, unhandled lower sync. Fix: leave ELR alone.

TTBR1 first cut (ADR-016): `TCR.EPD1` clear, T1SZ=25, `TTBR1_PRIV=0xFFFFFF8000000000`, one EL1-only L3 leaf. Kernel still identity at `0x40080000`. EL0 `LDR` → `ttbr1: no el0`.

42 tests. qemu-smoke ok. Umbrella isolation Planned. PAN unclaimed on cortex-a57.

Do not self-merge. GitHub author of #24 is expected `cursor[bot]`; merge hat is `artofdream`.
