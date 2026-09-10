# EL0 isolation (P-SEC-3 / ADR-013)

**Isolation: Planned.** A first mile exists. Do not claim userspace or isolation.

Direction: [ADR-013](../03-adr/ADR-013-el0-isolation-direction.md). Threat model: [security.md](security.md). Code: `src/el0.rs` (`is_active() == false`).

## What exists today

- Kernel runs at EL1 (`SPSel = 0`).
- One map-window page (`paging::EL0_PAGE`) can be UXN-clear / PXN for a trampoline.
- Lower-EL AArch64 **sync** handles `SVC #0` and a permission IABORT, then returns to EL1t. Other lower-EL slots still park ([ADR-004](../03-adr/ADR-004-el1-vbar-brk.md)).
- RAM identity maps set UXN, so an EL0 fetch of kernel `.data` is a permission fault when the first-mile probe runs.

## First-mile probes

| Probe | What closes it | Honesty |
| --- | --- | --- |
| EL0 entered and returned | Serial `el0: svc` + `el0: ok`; `#[test_case]` `el0_svc_roundtrip` | Entered and left. Not a user process. |
| EL0 cannot execute kernel data | Serial `el0: nx kernel`; `#[test_case]` `el0_cannot_execute_kernel_data` | UXN IABORT on `.data`. Not “cannot read.” Not a separate map. |

## Still Planned (isolation)

| Probe | What would close it |
| --- | --- |
| User map ≠ kernel map | A user VA that is not the kernel identity window (own `TTBR0` / ASID). |
| Cannot *read* kernel data from EL0 | PAN or an unmapped/privileged kernel window, with a serial marker. |
| Standing EL0 context | `is_active() == true` while a user task exists. |

Unprobed stays **Unknown**. Isolation stays **Planned** until those rows have probes. Do not say “EL0 works” or “isolated.”
