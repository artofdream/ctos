# ADR-013 — EL0 isolation direction (first mile)

- Status: Accepted (direction + first mile + user-TTBR0 read mile + ASID isolation mile + standing EL0; umbrella isolation still Planned)
- Date: 2026-09-10
- Updated: 2026-09-11 (standing EL0 dual-SVC; TTBR1 first cut is ADR-016; isolation still Planned)

## Context

Vision and [FR-11](../02-requirements/fr-nfr.md) leave userspace / EL0 as later work. [NFR-10](../02-requirements/fr-nfr.md) names EL0 isolation as Planned. [ADR-011](ADR-011-three-pillars.md) and the threat model ([security.md](../framework/security.md)) treat “malicious EL0” as a **named** adversary, not a standing userspace.

The first revision of this ADR recorded direction only (`is_active() == false`, lower-EL slots parked). The post-#18 deepen implements the **smallest honest mile** toward the closing probe. It does not invent a POSIX process model.

## Decision

1. **First mile (this tree).** Deliberate `ERET` to EL0t (DAIF masked) on one map-window page that is UXN-clear and PXN (EL1 must not fetch it). Two paths:
   - `SVC #0` taken on the lower-EL AArch64 **sync** slot; handler prints `el0: svc` and `ERET`s back to EL1t.
   - `BR X0` to a kernel `.data` bait; UXN on the identity image must take a lower-EL permission IABORT; handler prints `el0: nx kernel` and returns to EL1t.
   Serial `el0: ok` after the first-mile pair and the later miles on this path. The first mile itself does not install a standing context.
2. **Closing probe for “cannot execute kernel data.”** The UXN / translation IABORT *is* that instruction-fetch probe. Mark that **mile** Verified when the serial / `#[test_case]` pass.
3. **User TTBR0 window (this tree, post-#19).** A second L1 (`L1_USER`) is installed on `ERET` to EL0 (ASID=1 in TTBR0[63:48]). It maps kernel `.text`/`.rodata` and the exception stack so the lower-EL handler can restore kernel TTBR0 (kept in `TPIDR_EL1`), plus the shared map-window L2 for the trampoline. Coverage is **every 2 MiB** that holds those ranges, not only the first RAM 2 MiB. It **omits** `.data` / `.bss` / heap. An EL0 `LDR` from kernel `.data` is a lower-EL translation (or permission) DABORT (`el0: no kernel read`). The EL0 trampoline still `TLBI VMALLE1` because kernel `.data` leaves are **global** (`nG=0`); dropping that flush would leak a cached `.data` translation into the user ASID. Programming ASID=1 on that path is still not the isolation mile.
4. **ASID isolation mile (this tree).** Two EL1 TTBR0 values (kernel L1 + ASID 1 vs a clone L1 + ASID 2) map one window VA to different PAs with `nG=1`, and a second VA only under ASID 1. The switch is `MSR TTBR0` + `ISB` — **no** `TLBI VMALLE1`. Dual read must see the active ASID’s magic (`asid: dual`). The ASID-1-only VA must take a current-EL translation DABORT under ASID 2 (`asid: conflict`). Seeing ASID 1’s magic under ASID 2 is **Failed** (`asid: stale`). This is TLB ASID isolation on this virt guest, not “EL0 isolated.”
5. **Standing EL0 (this tree).** A bounded user context on the user TTBR0: `SVC #1` prints `el0: standing` and **ERETs back to EL0**; the user `MOVZ` must run; `SVC #2` prints `el0: restored` and returns to EL1t. `is_active()` is true only between install and teardown (saved user PC / SP / TTBR0). A trampoline-only flag is not this mile. Lower-EL IRQ / FIQ / SError still park (not exercised). Not a POSIX process. Not preemption.
6. **Isolation remains Planned.** Missing: PAN (typically unimplemented on `-cpu cortex-a57` / ARMv8.0 — read `ID_AA64MMFR1_EL1.PAN` before claiming it), full TTBR1 / higher-half identity teardown ([ADR-016](ADR-016-ttbr1-private-page.md) is the first cut only), EL0 entry without a full TLBI. Do not move FR IDs. Do not mint FR-16+. Do not claim PAN.
7. **Lower-EL slots.** AArch64 sync is live for SVC / IABORT / DABORT (including standing dual-SVC and the TTBR1 private-page load). Lower-EL IRQ / FIQ / SError and all AArch32 slots stay parks ([ADR-004](ADR-004-el1-vbar-brk.md)).
8. **Honesty.** Docs and PRs may say “EL0 entered and returned,” “EL0 cannot execute kernel data,” “EL0 cannot read kernel `.data`,” “user TTBR0 omits kernel data,” “standing EL0 context,” or “ASID isolation” when those probes pass. The TTBR1 private-page mile is [ADR-016](ADR-016-ttbr1-private-page.md). They may not say “EL0 works,” “userspace,” or “EL0 isolated.”

## Consequences

- [el0.md](../framework/el0.md) splits first-mile vs isolation rows.
- Preemption and SMP remain separate later ADRs ([ADR-010](ADR-010-cooperative-rr-el1.md)).
- This ADR does not claim Raspberry Pi or a POSIX process model.
