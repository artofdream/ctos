# ADR-006 — GICv2 and the EL1 physical timer

- Status: Accepted
- Date: 2026-09-09

## Context

[FR-08](../02-requirements/fr-nfr.md) requires hardware interrupts via the virt GIC, with a timer tick observable. Roadmap M5 is that path. [ADR-004](ADR-004-el1-vbar-brk.md) installed `VBAR_EL1`; [ADR-005](ADR-005-fatal-exception-stack.md) split stacks. First-level IRQ was still a park.

QEMU `-machine virt` can expose GICv2 or GICv3 (`gic-version=2|3|4|host|max`). The current smoke line is `-machine virt` with no `gic-version` override. QEMU 8.2’s `virt` default is **GICv2** (distributor at `0x0800_0000`, CPU interface at `0x0801_0000`). GICv3 uses a redistributor at `0x080A_0000` instead of that CPU interface.

The ARM generic timer is already in the core (`CNTP_*_EL0` / `CNTV_*_EL0`). No extra virtio device is required.

## Decision

1. **Program GICv2**, matching default `-machine virt`. Do not add a GICv3 redistributor driver in this milestone. If a later QEMU default flips to v3, pin `gic-version=2` on the smoke line or add a v3 driver in a new ADR.
2. **Use the non-secure EL1 physical timer** (`CNTP_CTL_EL0` / `CNTP_TVAL_EL0` / `CNTPCT_EL0`) and **PPI 30**. If `ensure_el1` drops from EL2, set `CNTHCTL_EL2.EL1PCTEN|EL1PCEN` and zero `CNTVOFF_EL2` so EL1 can use the counter and physical timer.
3. **Take IRQs on the current-EL / SP_EL0 bank** (vector offset `0x080`). The kernel runs with `SPSel = 0`, so first-level IRQs use `SP_EL1` (the M4 exception stack). Save the same GPR + ELR/SPSR/ESR frame as sync, then EOI.
4. **Keep DAIF.I masked except for an observe window.** Hello and `#[test_case]` unmask, wait for one tick (or a `CNTPCT` timeout), then remask and stop the timer so M3 `BRK` / M4 nested fatal are not interrupted.
5. **Raw UART on the IRQ path.** The handler must not take `spin::Mutex` (same reason as ADR-005). First tick prints `timer: tick`.

## Consequences

- `scripts/qemu-smoke.sh` requires `timer: tick` and rejects `timer: tick missed`, then still requires the M3/M4 strings.
- `#[test_case]` can wait for a tick with IRQs unmasked and continue.
- FIQ, SError, lower-EL, and unexpected IRQ IDs stay parks / raw markers. UART input is M6.
- This ADR does not claim Raspberry Pi, GICv3, virtualization=on as the primary path, or a taken FIQ.
