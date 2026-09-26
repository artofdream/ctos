# 2026-09-26 — ADR-088 M2 MMIO high alias (handoff)

- ADR: [ADR-088](../../docs/03-adr/ADR-088-mmio-high-alias.md). Stacked on #139.
- GIC / PL011 / virtio via TTBR1 Device blocks (`mmio_va`); identity MMIO L1 cleared; `ident: inv k=1 u=1 a=1 leaks=0` / `ident: inv-ok allow=stub`.
- Box `qemu-smoke: ok` ~14:43 CEST. b2 profile TCG boot OK; KVM re-run not done (≈ $0.2 if the sponsor wants B2-P re-proven on the new boot path).
- `wx.rs` "MMIO XN" check moved to `paging::mmio_xn` (live alias).
