# Session memory — 2026-09-12 (Track A / A8)

Docs-only A8 on `cursor/a8-documented-sample-apps-6be5`. Rebased onto `main` `c63077e` (B7 / ADR-029 + Darwin #50 + A7). A9 not included. Conflicts: kept A8 FAT Verified wording and B7 containers **non-goal**.

- No ADR: existing ADRs already decided the sample classes. A8 is how to rebuild them.
- Did not implement a new VFS-using hello. A7 suggested that as an A8 pickup; the sponsor brief said recipes only and “do not invent new runtime claims.” Documented the gap honestly (`libctos` has `fs_open`; hello does not call it).
- A9 leftover: one ELF still; need OS slot vs app slot + cross-update probe.
- Kernel / smoke scripts untouched. Preserve A1–A7 markers by not editing them.

Do not treat this file as the honesty ledger.
