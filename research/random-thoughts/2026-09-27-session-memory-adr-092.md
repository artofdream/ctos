# Session memory — 2026-09-27 ADR-092 (G1/G2)

- #136–#138 merged by the sponsor 2026-09-26 (merge commits e4f9cb6 / fc80df7 / c1d98c7). Branches auto-deleted. #139 auto-retargeted to `main` (clean). Open stack: #139 → #140 → #141 → #142 → #143 → ADR-092 PR.
- Serial log lines end in `\r`: smoke regexes need `[[:space:]]*$`, not `$`.
- The smoke builds leak variants from the working tree mid-run. Don't edit `src/` while it runs.
- GNU `grep -E` supports `\1`, so `far=\1` pins FAR == VA even under `/bin/sh`.
- EL0 exec permission = UXN, not AP[1]. `l3_page_el0_exec` is AP = 00 + UXN clear (execute-only).
- PAN blocks EL1 access through EL0-accessible mappings. Fill user pages via the TTBR1 RAM alias (`to_high_va(pa)`).
- G4 follow-up idea: in `user_range_ok_max`, require `AP[1]` (and `!AP[2]` for writes) on the user leaf. The loader must map rodata EL0-R (AP = 11, UXN clear for the shared RX page).
