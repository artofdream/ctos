# ADR-048 — App hosting “done” claim criteria

- Status: Accepted (claim gate). Product “apps run independently / hosting done” stays **Planned**. Do **not** claim done now.
- Date: 2026-09-13

## Context

Track A children A1–A9 have first cuts (ABI, CRT, loader, standing-as-normal, isolation miles, memfs, FAT, recipes, OS/app slots + cross-update / embed-off). The honesty ledger and roadmap correctly refuse to round those miles up to “app hosting is done.” Sponsors still need **explicit criteria** so a future Verified claim is fail-closed rather than mood.

Related: [ADR-030](ADR-030-os-app-slots.md), [ADR-032](ADR-032-track-a-leftovers.md), [ADR-029](ADR-029-containers-nongoal.md), [hosting-apps.md](../overview/hosting-apps.md), [ADR-047](ADR-047-isolation-leftovers-decisions.md) (isolation leftovers are separate — closing them is **not** required to meet these criteria, and meeting these criteria does **not** make “EL0 isolated” true).

## Decision

### A1–A9 first cuts

Treat A1–A9 **first cuts** as **Verified** when the honesty ledger has matching rows for this tip (or cited merge tips). That is **not** product app hosting.

### Product claim stays Planned until all criteria below are met

The sentence **“apps run independently of the OS”** / **“app hosting is done”** may move from Planned to Verified **only** when **every** criterion holds, with named probes in the honesty ledger:

1. **Documented recipes** — Rebuild / run recipes for freestanding apps cite only existing serial markers and stay buildable (`./scripts/docs-build.sh` + A8 hub). No invented runtime.
2. **Slot cross-update** — Same published app ELF loads on this OS **and** a documented prior OS (`ba6541c` or a later sponsor-pinned prior) with fail-closed host smoke (`cross-update … slot:ok` both boots). [ADR-032](ADR-032-track-a-leftovers.md).
3. **No production embed** — A2–A4 / production slot path must not `include_bytes!` the hello app. FAT `/hello` (or an equivalent published slot file) is the production payload. Probe-only embeds (e.g. ADR-046 measurement) must never print `slot: embed` and must stay out of the production path.
4. **Honesty about missing Linux / POSIX / containers** — Docs and PRs must still say: not a Linux ABI, not POSIX, not glibc/musl, guest OCI/Docker/k8s remains **non-goal** ([ADR-029](ADR-029-containers-nongoal.md)). Meeting (1)–(3) does **not** authorize those claims.
5. **Explicit sponsor accept** — A PR that flips the product row to Verified must cite this ADR’s checklist and must not self-merge ([ADR-002](ADR-002-pr-identity-split.md)).

### What does **not** satisfy the claim

- A1–A9 first cuts alone
- Isolation ladder miles / ADR-047 decisions
- Host `docker-smoke.sh` (Docker hosts ctos, not the reverse)
- Slot CNTPCT pairs / “slots are free” marketing ([ADR-046](ADR-046-slot-perf-delta.md))
- Linux-compat research (Track B) or any “runs Alpine” sentence

## Honesty

Say “A1–A9 first cuts exist” when the ledger rows say so. Do **not** say “app hosting is done,” “apps update independently,” “OTA,” or “container host” until the criteria above are Verified — and containers stay non-goal regardless.

## Consequences

- Ledger row “Track A app hosting (slots)” stays **Planned** and cites this ADR for the claim gate.
- [hosting-apps.md](../overview/hosting-apps.md), Track A / roadmap, NFR-10 text point here.
- No new FR/NFR IDs. Do not mint NFR-15+.

## Progress checklist

Living Met/Partial/Blocked table (tip evidence): [hosting-apps.md — Claim criteria](../overview/hosting-apps.md#claim-criteria-adr-048) and [honesty-ledger.md — ADR-048 progress](../framework/honesty-ledger.md#adr-048-progress-checklist). Product claim stays **Planned** until criterion 5 (sponsor accept).

