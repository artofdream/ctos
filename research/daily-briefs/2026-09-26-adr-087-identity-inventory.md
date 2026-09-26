# 2026-09-26 — ADR-087 M1 identity inventory ratchet (handoff)

- ADR: [ADR-087](../../docs/03-adr/ADR-087-identity-inventory-ratchet.md). Stacked on #138.
- Tears: I2 low RAM (128 pages), I4 image padding tail (270 kernel + 270 user pages on the hello build), I5 pre-heap frame (1 page). Fault probes `ident: low/tail/kend-fault`.
- Ratchet: `ident: inv k=2 u=1 a=2 leaks=0` + `ident: inv-ok allow=mmio,stub`. In-boot plant caught (`ident: inv-neg k/u caught`, `clean`). `inv-leak-probe` kernel caught by smoke.
- Box `qemu-smoke: ok` 14:35–14:40 CEST. b2 profile TCG boot OK (not KVM).
- Surprise: the `.rodata` pointer rewrite relocated the allowlist constants.
- Next: M2 MMIO high alias (D3 accepted) → allowed set {stub}.
