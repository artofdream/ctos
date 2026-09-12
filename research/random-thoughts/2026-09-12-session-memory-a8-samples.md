# Session memory — 2026-09-12 (Track A / A8)

Docs-only A8 on `cursor/a8-documented-sample-apps-6be5`. Rebased onto `main` `e347983` (A7 + Darwin docs-build #50). A9 not included.

- No ADR: existing ADRs already decided the sample classes. A8 is how to rebuild them.
- Did not implement a new VFS-using hello. A7 suggested that as an A8 pickup; the sponsor brief said recipes only and “do not invent new runtime claims.” Documented the gap honestly (`libctos` has `fs_open`; hello does not call it).
- A9 leftover: one ELF still; need OS slot vs app slot + cross-update probe.
- Kernel / smoke scripts untouched. Preserve A1–A7 markers by not editing them.

Do not treat this file as the honesty ledger.
