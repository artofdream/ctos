# 2026-11-12 — ADR-112: real-hardware SError plan (docs only)

- Request: the user, 2026-11-12 22:15 CET, "proceed with next milestone including real-hardware SError".
- Base: `main` `9ebef21` (#170). One docs-only draft PR. No AWS launch, no code.
- Finding: on Graviton metal, the only SError ctos can take on demand is a **virtual SError** that the hardware delivers after KVM sets `HCR_EL2.VSE` / `VSESR_EL2`. That was already shown (ADR-085/095/099) with KVM's default syndrome.
- Ruled out for a guest: physical SError via unmapped IPA (QEMU ignores the MMIO result: reads as zero), via `ERX*` pseudo-faults (KVM traps them, RAZ/WI), via host pseudo-faults (UC panics the host; UEU/UER reach the guest as a **virtual SError**; Neoverse V1 injects only CE/DE/UC), via EINJ (comes back as a synchronous abort). Bare-metal ctos: Graviton boots UEFI + ACPI only; a separate port.
- Chosen: HW-VSE. KVM **virtual SError** with a host-chosen ESR (`serror_has_esr`), three rows (nonce, RAS-coded, KVM default), read back from `ESR_EL1` in ctos; plus read-only host RAS inventory.
- Cost: last seen Spot `c7g.metal` eu-north-1 $0.2476/h (Sep 2026); live read failed (connector credentials expired). ≈ 10–12 min per run, ≈ $0.05–0.06 expected, $0.33 cap, ≤ $0.70 with one retry.
- Open decisions D1–D4 are listed in [ADR-112](../../docs/03-adr/ADR-112-graviton-hardware-virtual-serror-esr.md).
