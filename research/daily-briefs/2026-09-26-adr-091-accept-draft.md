# 2026-09-26 — ADR-091 (M5): umbrella accept DRAFT (sponsor D4)

- **Status:** Draft / pending sponsor accept. **B3 not Met.** Umbrella “EL0 isolated” = Planned / non-claim. Nothing flipped.
- **What the draft holds:** the exact proposed sentence; scope S1–S7 with markers; ADR-090 §1 quoted verbatim (checked byte-identical); non-scope (no KPTI / side channels, QEMU TCG only, one core, no task-vs-task beyond ASID, no DMA/IOMMU/EL2, no certification, stock QEMU still parks); M6 prerequisites.
- **Found while drafting (for the sponsor):**
  - G1: no exhaustive EL0-permission walk. The read/exec probes are single-address, and the rest holds by construction: kernel builders always set UXN and never AP[1], checked by code reading.
  - G2: no EL0 write probe.
  - G3: optional B2-P KVM re-verify, ≈ $0.2.
- **Sponsor options:** (a) accept as drafted with G1–G3 as known limits; (b) require G1/G2 first (M5b); (c) narrow the sentence.
- **M6 (not started):** sponsor's written accept; D5 branch protection requiring the 3 checks (`QEMU aarch64 smoke (ubuntu-24.04-arm)`, `QEMU aarch64 smoke (ubuntu-24.04)`, `QEMU NMI pin smoke (taken SError)`); merge #136→#143 in order; the flip PR.
