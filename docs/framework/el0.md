# EL0 isolation (P-SEC-3 / ADR-013)

**Isolation: Planned.** A first mile, a user-TTBR0 read mile, and an ASID TLB mile exist. Do not claim userspace or “EL0 isolated.”

Direction: [ADR-013](../03-adr/ADR-013-el0-isolation-direction.md). Threat model: [security.md](security.md). Code: `src/el0.rs` (`is_active() == false`), `src/asid.rs`.

## What exists today

- Kernel runs at EL1 (`SPSel = 0`).
- One map-window page (`paging::EL0_PAGE`) can be UXN-clear / PXN for a trampoline.
- `ERET` to EL0 switches to a **user TTBR0** (`L1_USER`, ASID=1) that maps kernel text/rodata + the exception stack + the trampoline window (every 2 MiB those ranges occupy), and **omits** `.data` / `.bss` / heap. The lower-EL handler restores kernel TTBR0 from `TPIDR_EL1` before touching kernel data. That path still `TLBI VMALLE1` (kernel `.data` leaves are global).
- Dual EL1 ASIDs (1 vs 2) with `nG` probe pages switch **without** `TLBI VMALLE1` (`src/asid.rs`).
- Lower-EL AArch64 **sync** handles `SVC #0`, a kernel-data IABORT, and a kernel-data DABORT, then returns to EL1t. Other lower-EL slots still park ([ADR-004](../03-adr/ADR-004-el1-vbar-brk.md)).

## Probed miles

| Probe | What closes it | Honesty |
| --- | --- | --- |
| EL0 entered and returned | Serial `el0: svc` + `el0: ok`; `#[test_case]` `el0_svc_roundtrip` | Entered and left. Not a user process. |
| EL0 cannot execute kernel data | Serial `el0: nx kernel`; `#[test_case]` `el0_cannot_execute_kernel_data` | IABORT on `.data` (UXN and/or unmapped). Not isolation. |
| User TTBR0 omits kernel `.data` | Walk `L1_USER`; `#[test_case]` `user_ttbr0_omits_kernel_data` | Distinct user table. Text is still mapped so the handler can run. |
| EL0 cannot *read* kernel `.data` | Serial `el0: no kernel read`; `#[test_case]` `el0_cannot_read_kernel_data` | Translation/permission DABORT. Not PAN. Not a standing user. |
| ASID field programmed | `user_ttbr0() >> 48 == 1` | Programming fact on the EL0 trampoline. Not the isolation mile. |
| ASID isolation | Serial `asid: dual` / `asid: conflict` / `asid: ok`; `#[test_case]` `asid_isolation_without_vmalle1` | Dual TTBR0 without `TLBI VMALLE1`. Stale ASID-1 data under ASID 2 is Failed. Not “EL0 isolated.” |

## Still Planned (isolation)

| Probe | What would close it |
| --- | --- |
| PAN | `ID_AA64MMFR1_EL1.PAN != 0` **and** an EL1 access to an EL0-accessible page faults. `-cpu cortex-a57` is ARMv8.0 — usually unimplemented. Do not claim PAN. |
| TTBR1 / higher-half | Kernel only in the high VA range. |
| Standing EL0 context | A real standing user task with `is_active() == true` for that lifetime. **Deferred:** a trampoline flag is not a user; lower-EL IRQ still parks; no POSIX. |
| EL0 entry without full TLBI | User TTBR0 switch that does not `TLBI VMALLE1` (needs `nG` on kernel `.data` or an ASID-specific invalidate). |

Unprobed stays **Unknown**. The umbrella isolation row stays **Planned** until PAN + standing EL0 + TTBR1 have probes. Do not say “EL0 works” or “EL0 isolated.”
