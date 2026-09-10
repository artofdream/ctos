# ADR-013 — EL0 isolation direction (scaffold)

- Status: Accepted (direction only)
- Date: 2026-09-10

## Context

Vision and [FR-11](../02-requirements/fr-nfr.md) leave userspace / EL0 as later work. [NFR-10](../02-requirements/fr-nfr.md) names EL0 isolation as Planned. [ADR-011](ADR-011-three-pillars.md) and threat-model v1 ([security.md](../framework/security.md)) treat “malicious EL0” as a **named future adversary**, not a current one.

This ADR records the **direction** so later sessions do not invent a different isolation story. It does **not** implement EL0.

## Decision

1. **Not this PR’s kernel work.** No `ERET` to EL0, no `SPSR_EL1` user mode, no user page tables, no syscalls. `src/el0.rs` is a stub (`is_active() == false`). Lower-EL `VBAR_EL1` slots keep parking ([ADR-004](ADR-004-el1-vbar-brk.md)).
2. **Closing probe (later, not claimed here):** a lower-EL instruction fetch of kernel data (or an EL0 map of a kernel page with PXN clear / UXN clear) must fault, with a serial marker and a `#[test_case]`. Until that probe exists the ledger row stays **Planned**.
3. **Roadmap:** P-SEC-3 stays Planned. Do not move FR IDs. Do not mint FR-16+.
4. **Likely later ingredients** (not decided here): `TTBR0` user vs kernel split or a dedicated user window, ASID, `UXN` on kernel pages (already set on the ADR-012 RAM map), a separate user stack, and an SVC path. Each of those is its own ADR when started.
5. **Honesty.** Docs and PRs may say “EL0 direction written.” They may not say “EL0 works,” “userspace,” or “isolated.”

## Consequences

- [el0.md](../framework/el0.md) lists Planned probes. The stub must keep compiling (`cargo build` / `cargo test`).
- Preemption and SMP remain separate later ADRs ([ADR-010](ADR-010-cooperative-rr-el1.md)).
- This ADR does not claim Raspberry Pi or a POSIX process model.
