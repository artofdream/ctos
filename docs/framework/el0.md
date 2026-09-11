# EL0 isolation (P-SEC-3 / ADR-013)

**Isolation: Planned.** A first mile, a user-TTBR0 read mile, an ASID TLB mile, a standing EL0 context, a TTBR1 private-page first cut, an EL1 high-VA fetch mile, and an identity-tear first cut exist. Do not claim userspace or “EL0 isolated.”

Direction: [ADR-013](../03-adr/ADR-013-el0-isolation-direction.md). TTBR1 first cut: [ADR-016](../03-adr/ADR-016-ttbr1-private-page.md). EL1 fetch mile: [ADR-017](../03-adr/ADR-017-ttbr1-high-el1-exec.md). Identity-tear first cut: [ADR-018](../03-adr/ADR-018-identity-teardown-first-cut.md). Threat model: [security.md](security.md). Code: `src/el0.rs`, `src/asid.rs`, `src/ttbr1.rs`, `src/teardown.rs`.

## What exists today

- Kernel runs at EL1 (`SPSel = 0`).
- One map-window page (`paging::EL0_PAGE`) can be UXN-clear / PXN for a trampoline or standing payload.
- `ERET` to EL0 switches to a **user TTBR0** (`L1_USER`, ASID=1) that maps kernel text/rodata + the exception stack + the trampoline window (every 2 MiB those ranges occupy), and **omits** `.data` / `.bss` / heap. The lower-EL handler restores kernel TTBR0 from `TPIDR_EL1` before touching kernel data. That path still `TLBI VMALLE1` (kernel `.data` leaves are global).
- Dual EL1 ASIDs (1 vs 2) with `nG` probe pages switch **without** `TLBI VMALLE1` (`src/asid.rs`).
- A bounded **standing** user context on that TTBR0: `SVC #1` stays at EL0 (`el0: standing`), user `MOVZ` runs, `SVC #2` restores EL1 (`el0: restored`). `is_active()` is true only for that lifetime.
- TTBR1 walks are enabled. One kernel-private high page (`TTBR1_PRIV`) is EL1-only; EL0 load faults. Identity RAM is aliased at `va + TTBR1_BASE` via **cloned** RAM tables; EL1 can fetch a real path there and `VBAR_EL1` is the high alias. One dedicated identity text page is unmapped from TTBR0 (`ident: ok`). `_start` / QEMU `-kernel` stay at `0x4008_0000`. Full identity teardown is Planned.
- Lower-EL AArch64 **sync** handles `SVC`, a kernel-data IABORT, a kernel-data DABORT, and the TTBR1 private-page DABORT, then returns to EL1t (or stays at EL0 on standing `SVC #1`). Other lower-EL slots still park ([ADR-004](../03-adr/ADR-004-el1-vbar-brk.md)).

## Probed miles

| Probe | What closes it | Honesty |
| --- | --- | --- |
| EL0 entered and returned | Serial `el0: svc` + `el0: ok`; `#[test_case]` `el0_svc_roundtrip` | Entered and left. Not a user process. |
| EL0 cannot execute kernel data | Serial `el0: nx kernel`; `#[test_case]` `el0_cannot_execute_kernel_data` | IABORT on `.data` (UXN and/or unmapped). Not isolation. |
| User TTBR0 omits kernel `.data` | Walk `L1_USER`; `#[test_case]` `user_ttbr0_omits_kernel_data` | Distinct user table. Text is still mapped so the handler can run. |
| EL0 cannot *read* kernel `.data` | Serial `el0: no kernel read`; `#[test_case]` `el0_cannot_read_kernel_data` | Translation/permission DABORT. Not PAN. |
| Standing EL0 context | Serial `el0: standing` / `el0: restored`; `#[test_case]` `standing_el0_enter_leave` | Real enter/leave on user TTBR0. `is_active()` true only while standing. Not POSIX. Lower-EL IRQ still parks. |
| ASID field programmed | `user_ttbr0() >> 48 == 1` | Programming fact on the EL0 trampoline. Not the isolation mile. |
| ASID isolation | Serial `asid: dual` / `asid: conflict` / `asid: ok`; `#[test_case]` `asid_isolation_without_vmalle1` | Dual TTBR0 without `TLBI VMALLE1`. Stale ASID-1 data under ASID 2 is Failed. Not “EL0 isolated.” |
| TTBR1 private page | Serial `ttbr1: el1` / `ttbr1: no el0` / `ttbr1: ok`; `#[test_case]` `ttbr1_el1_sees_priv_el0_does_not` | EL1-only high page. Not a relocated kernel. |
| EL1 fetch from TTBR1 high VA | Serial `ttbr1: el1 exec` / `ttbr1: vbar`; `#[test_case]` `el1_executes_from_ttbr1_high_va` | Real EL1 path + high VBAR. Identity boot stub stays. |
| Identity-tear first cut | Serial `ident: split` / `ident: fault` / `ident: high` / `ident: no el0` / `ident: ok`; `#[test_case]` `identity_tear_el1_faults_high_stays` | One identity text page unmapped; high twin still fetches. Not a relocated kernel. Full teardown Planned. |

## Still Planned (isolation)

| Probe | What would close it |
| --- | --- |
| PAN | `ID_AA64MMFR1_EL1.PAN != 0` **and** an EL1 access to an EL0-accessible page faults. `-cpu cortex-a57` is ARMv8.0 — usually unimplemented. Do not claim PAN. |
| Full higher-half / identity teardown | Remaining identity `.text`/`.data`/heap unmapped after a complete high-VA jump ([ADR-018](../03-adr/ADR-018-identity-teardown-first-cut.md) is one torn text page + split tables, not the full tear). |
| EL0 entry without full TLBI | User TTBR0 switch that does not `TLBI VMALLE1` (needs `nG` on kernel `.data` or an ASID-specific invalidate). |
| Lower-EL IRQ while standing | Timer (or other) IRQ taken from EL0 and returned. Still parked. |

Unprobed stays **Unknown**. The umbrella isolation row stays **Planned** until PAN + full identity teardown have probes (standing + TTBR1 first cut + EL1 high-VA fetch + one torn text page are not enough). Do not say “EL0 works,” “EL0 isolated,” or “the kernel moved.”
