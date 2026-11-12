# 2026-11-12 — ADR-112 B3: real-hardware SError on Graviton metal (plan, no launch)

- Research: as a KVM guest, ctos can only see a **virtual SError**. KVM handles physical SErrors at EL2 and at most re-injects a virtual one, and guest `ERX*` (incl. `ERXPFGCTL_EL1`) are RAZ/WI. A physical SError needs bare-metal ctos plus EINJ / `ERXPFG*`, which is not available today.
- Chosen (option A): host-chosen-syndrome virtual SError (`serror_has_esr`, per-run nonce ISS) that the `b2-serror` profile must read back exactly in `ESR_EL1`; rows N (no inject), D (default ESR), E (nonce ESR); read-only RAS / EINJ inventory.
- Cost: `c7g.metal` Spot eu-north-1 $0.2476/h (public feed, 2026-11-12); about 20 min billed ≈ $0.09 per run; hard cap $0.43 per run. No AWS call made. See [ADR-112](../../docs/03-adr/ADR-112-b3-graviton-real-hardware-serror-plan.md).
