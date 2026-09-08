# ADR-001 — Apply a honesty/harness practice to ctos

- Status: Accepted
- Date: 2026-09-08

## Context

ctos is a bare-metal kernel. Agents and humans will extend it across sessions. Status words drift: "it boots" gets written because source exists, or because a README says `cargo run`. Without a probe, that is fiction.

Harness-engineering practice (guides, sensors, a tight loop, persistent memory, no self-merge, observability of claims) is useful here. Prior art: [architecture.artof.link](https://architecture.artof.link/). This ADR does **not** adopt that site's product, florist domain, or stakeholder roster.

## Decision

Apply a **ctos-native** honesty/harness practice:

1. **Honesty** — every status word is a claim; the [honesty ledger](../framework/honesty-ledger.md) records the probe. Unprobed = Unknown. Never round Unknown up to Verified.
2. **Document-first** — vision, architecture, ADRs, and roadmap land before more kernel features.
3. **Second brain** — four vaults under `research/` (procedure via skills, correction via constraints/rules, relationship via doc links, daily brief / session memory).
4. **Thin roles** — Knowledge Guardian, Coherence Guardian, Kernel Engineer, MR Coordinator (`ctos-*` skills only).
5. **Loop** — one milestone → one branch → one GitHub PR. The author does not self-approve.
6. **Sensors over vibes** — when a failure repeats, strengthen a sensor or gate (see [antifragility.md](../framework/antifragility.md)).

Tracker and reviews stay on **GitHub** (`gh`). No GitLab workflow is part of this repo.

## Consequences

- README and docs may teach `cargo run` without claiming QEMU boot is Verified.
- New kernel work cites a roadmap milestone and updates the ledger when something is actually probed.
- Role names stay `ctos-*`. Do not introduce `aea-*` hats or shop case-study content.
