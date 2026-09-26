# 2026-09-26 — ADR-086 umbrella “EL0 isolated” checklist audit (handoff)

- ADR: [ADR-086](../../docs/03-adr/ADR-086-el0-isolated-checklist-audit.md). Docs-only. Stacked on #137 (`915796f`); #136 / #137 unmerged at write time.
- Result: umbrella **Planned / non-claim**. §A 13/13 miles + PAN enable Verified this session. §B 0/3 Met.
- No AWS spend in this phase. No CloudAgent.

## Probes this session

- Agent box 13:59–14:04 CEST: `scripts/qemu-smoke.sh` on `915796f`, QEMU 10.0.13 (Debian), rustc 1.100.0-nightly `0fc141305` (repo pin; corrected 2026-09-26): `qemu-smoke: ok`, force-fail exit 1, park `inject-nmi=>error:machine does not provide NMIs` → `el0: serror-park`, `ident: start-stay lo=0x40080000 hi=0x40081000`, `pan: enabled` / `pan: el1-fault`.
- GHA PR run 36239619910 (13:41–13:53 CEST): default jobs success. `qemu-nmi-pin` job 108397549350 took bare `el0: serror`.
- Scratch TTBR0 walker (box ~14:10 CEST, not committed to `src/`): [inventory](2026-09-26-adr-086-ttbr0-inventory.txt), [patch](2026-09-26-adr-086-ttbr0-inventory-scratch.patch).

## Residual identity (kernel TTBR0)

1. `0x0–0x4000_0000` Device block (GIC / PL011 / virtio). 2. `0x4000_0000–0x4008_0000` low RAM (DTB hole, unused). 3. `0x4008_0000` stub page RO+X (start-stay). 4. Padding tail `[.ident_tear end, 0x4020_1000)` RO+X. 5. `__kernel_end` page RW. User TTBR0 keeps 3 and 4 (EL1-only).

## Next

M1 = committed `ident: inv` ratchet with allowlist {MMIO block, stub} + tear 2, 4, 5 (and 4 in user TTBR0). No sponsor gate. Sponsor decisions D1 (SError evidence class), D2 (`_start` vs “full teardown”), D3 (MMIO in scope), D4 (accept sentence), D5 (required checks).
