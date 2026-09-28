# Session memory — ADR-087 M1 (2026-09-26)

- The rodata pointer rewrite (ADR-020/025) is a loaded gun: it rewrote `INV_ALLOW`'s `0x4008_0000` to the TTBR1 alias, so the stub page failed its own allowlist. Anything security-relevant that holds a kernel-identity-looking constant in `.rodata` must be compared through `identity_pa()` or kept as an immediate. Worth an audit of other const tables later.
- The negative probe is cheap because the walker reads tables from memory, not via the TLB. Planting an entry the CPU never touches does not need a TLBI to be visible to the walk (one TLBI after removal, for hygiene).
- ASID-B shares the kernel RAM L2, so every kernel identity plant also shows up under `a`. Good cross-check.
- Status word discipline: the b2 profile was only TCG-booted after M1, not KVM-verified. Said so.
