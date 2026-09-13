# 2026-09-13 — ADR-042 isolation leftover honesty wrap

- Tip base: `75feef9` (ADR-041 on main).
- Mile: serial `ident: start-stay` + `ident: ram-stay`; `#[test_case]` `identity_boot_stub_stays_while_live_torn`; ADR-042 status table (Verified ADR-037…041 vs Planned leftovers).
- Still Planned: PAN enable on cortex-a57; lower-EL FIQ/SError; full identity teardown including `_start`; umbrella “EL0 isolated” (P-SEC-3l).
- Do not switch `-cpu`; do not yank `_start`; do not claim “EL0 isolated.”
- (evo-x2) intent `f074e48c-…`; CloudAgent HELD — work on agent box clone.
- Do not self-merge.
