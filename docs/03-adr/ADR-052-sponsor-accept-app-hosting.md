# ADR-052 — Sponsor accept: product app-hosting claim (ADR-048 #5)

- Status: Accepted (docs). Product “apps run independently of the OS” / “app hosting is done” is **Verified** under [ADR-048](ADR-048-app-hosting-claim-criteria.md) criteria 1–5. Not Linux userspace. Not POSIX. Not containers. Not “EL0 isolated.”
- Date: 2026-09-13

## Context

[ADR-048](ADR-048-app-hosting-claim-criteria.md) gated the product Track A app-hosting sentence on five criteria. On tip `main` @ `be789c5` (and newer), criteria **1–4 were already Met** in the honesty ledger (documented A8 recipes; A9 cross-update on this OS + `ba6541c`; no production hello embed — probe-only `include_bytes!` in `src/slot.rs` for [ADR-046](ADR-046-slot-perf-delta.md); honesty that Linux/POSIX/guest OCI remain refused / non-goal per [ADR-029](ADR-029-containers-nongoal.md)). Criterion **5** (explicit sponsor accept on the flip PR, no self-merge) was the remaining Blocked gate.

Sponsor signal (user): after a still-Planned review that listed product app hosting as blocked only on criterion #5, the sponsor said **“address 1. app hosting.”** Treat that as **explicit sponsor accept** for ADR-048.

Related: [ADR-048](ADR-048-app-hosting-claim-criteria.md), [ADR-030](ADR-030-os-app-slots.md), [ADR-032](ADR-032-track-a-leftovers.md), [ADR-029](ADR-029-containers-nongoal.md), [ADR-002](ADR-002-pr-identity-split.md), [hosting-apps.md](../overview/hosting-apps.md), [honesty-ledger.md](../framework/honesty-ledger.md#adr-048-progress-checklist).

## Decision

1. **Accept the product claim.** Flip the honesty-ledger row “Track A app hosting (slots)” from Planned → **Verified** for the scoped sentence: freestanding apps load from a separate OS/app slot (FAT `/hello`), rebuild recipes exist, the same published app ELF cross-updates on this OS and documented prior `ba6541c`, and production does not embed the hello payload.
2. **Cite this ADR + ADR-048 checklist** on the flip PR. Do **not** self-merge ([ADR-002](ADR-002-pr-identity-split.md)).
3. **Keep the non-claims.** Meeting ADR-048 does **not** authorize: Linux userspace, POSIX, glibc/musl, guest OCI/Docker/k8s, “EL0 isolated,” PAN enabled, taken SError Verified, “immutable OS” marketing, or OTA.
4. **Threat-model patch.** [security.md](../framework/security.md) may record the Verified product claim under ADR-048/052 and bump the patch version; it still must refuse the non-claims above.

## Honesty

Say the product Track A app-hosting claim is **Verified under ADR-048** (criteria Met×5, sponsor accept cited). Do **not** say ctos hosts Linux programs, POSIX apps, or containers. Do **not** round A1–A9 miles alone into the product claim without this accept record.

## Consequences

- Ledger product row + ADR-048 progress checklist criterion #5 → **Met** / product **Verified**.
- Present-tense docs ([hosting-apps.md](../overview/hosting-apps.md), Track A / roadmap, [syscall.md](../framework/syscall.md), NFR-10 note, companions) align without Linux/container/isolation round-ups.
- [ADR-048](ADR-048-app-hosting-claim-criteria.md) status amended: claim gate Accepted; product claim **Verified** via this ADR.
- No `src/` change required for this accept. No new FR/NFR IDs.
