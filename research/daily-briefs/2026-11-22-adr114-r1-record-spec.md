# 2026-11-22 — ADR-114 R1: record format + golden files (spec only)

- Why: the user said "proceed with R1" after ADR-114 (#178) merged as Proposed.
- What: [R1 spec](../../docs/03-adr/ADR-114-r1-record-spec.md). Input `/workin`: `\n`-separated lines, record `id,value` (id 1–8 ASCII digits kept verbatim, value optional `-` + 1–10 digits in `i32`). Everything else, including CRLF, spaces, `+`, NUL / non-ASCII, overflow, and a truncated final line, is a reject, never a crash. Output `/workout`: `id,canonical-value,crc16\n` per valid record (CRC-16/CCITT-FALSE, 4 lowercase hex) plus trailer `ok=<n> rejected=<m>\n`; complete only if the trailer matches. Whole-run fail tokens: `input_missing`, `input_empty`, `input_too_big`, `output_too_big`, `io`.
- Golden files: `tests/fixtures/adr114/workin` (505 B, 33 ok / 22 rejected, six records straddle a 64-byte read, one 73-byte line) and `workout` (447 B). Both fit `FILE_MAX` 1 KiB. `scripts/adr114-golden.py --check` is a host-side oracle, not wired into a required check.
- Not here: app, `libctos` ops, markers (R2); negatives incl. **virtual SError** (R3); soak / limits / restart (R4); any measurement or threshold (sponsor, after a baseline).
- Ledger: new R1 row **Planned** (spec only); ADR-114 row unchanged (Planned). Docs + fixtures only; no cloud. Research / teaching kernel, not a "secure OS".
