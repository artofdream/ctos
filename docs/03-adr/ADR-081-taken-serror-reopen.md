# ADR-081 — Taken lower-EL SError reopen (ADR-053 hard-stop re-probe; still blocked)

- Status: Accepted (docs / evidence). [#125](https://github.com/artofdream/ctos/issues/125) mile (3). Dated re-probe on default `-cpu cortex-a76` + documented virt options: **no honest inject** delivers ARM async SError to standing EL0 on QEMU 10 `virt` TCG. Taken path stays **deferred / non-goal (locked)**. `el0: serror-park` stays **Verified**. Guest A-clear / EXPECT plumbing stays **dormant prep**. Do **not** mark taken SError Verified.
- Date: 2026-09-19
- Tracks [#125](https://github.com/artofdream/ctos/issues/125). Builds on [ADR-053](ADR-053-taken-serror-hard-stop.md) reopen gate. CloudAgent **HELD** — agent-box / EVO-X2 / GHA only.

## Context

[ADR-053](ADR-053-taken-serror-hard-stop.md) hard-stopped taken lower-EL SError on QEMU 10 `virt` + `-cpu cortex-a57` after `inject-nmi` / HMP `nmi` → `machine does not provide NMIs`. Reopen gate §4: (a) honest host inject on the **accepted** smoke machine **or** sponsor-approved machine/CPU/GIC change with its own ADR; (b) serial `el0: serror` under EXPECT; (c) new ADR with working evidence.

[ADR-079](ADR-079-pan-cpu-reopen.md) / [ADR-080](ADR-080-pan-enable-fault.md) changed the default probe CPU to `cortex-a76` and Verified PAN enable — **not** an SError unlock. Sponsor / DSO unlocked mile (3): re-research any honest path; if none, document the blocker explicitly rather than invent Verified.

Do **not** invent FIQ / BRK / FEAT_NMI as SError. Do **not** silent machine invent. Keep `_start` at `0x4008_0000`. Umbrella “EL0 isolated” stays non-claim ([ADR-055](ADR-055-el0-isolated-checklist.md) / [ADR-060](ADR-060-isolation-leftovers-closure-checklist.md)).

## Research (agent-box, 2026-09-19 CEST)

Host: agent-box. `qemu-system-aarch64` **10.0.13** (Debian `1:10.0.13+ds-0+deb13u1`). Accel: **TCG only** (`query-kvm` → `present=false`; `-accel kvm` → `invalid accelerator kvm` on this box). Full brief: [research/daily-briefs/2026-09-19-adr-081-serror-inject-probe.md](../../research/daily-briefs/2026-09-19-adr-081-serror-inject-probe.md).

| Config | QMP `inject-nmi` | Notes |
| --- | --- | --- |
| `virt` + `cortex-a76` (**default smoke**) | **error** `machine does not provide NMIs` | HMP `nmi` same text |
| `virt` + `cortex-a57` (historical) | same | Matches ADR-053 / ADR-045 |
| `virt,gic-version=3` + `cortex-a76` | same | GICv3 alone ≠ TYPE_NMI |
| `virt,gic-version=3` + `max` | same | |
| `virt,virtualization=on` + `cortex-a76` | same | EL2 virt ≠ SError inject |
| `virt,gic-version=3,virtualization=on` + `cortex-a76` | same | |
| `virt,gic-version=3,secure=on` + `cortex-a76` | same | |
| `-cpu max,nmi=on` | **startup fail** | `Property 'max-arm-cpu.nmi' not found` |

`query-commands` lists `inject-nmi`, but virt does **not** implement `TYPE_NMI`. CXL `*-inject-*` commands are **not** an ARM SError path. Live smoke still arms `el0: serror-arm`, logs the QMP error, and prints `el0: serror-park` (no `el0: serror`).

### Options (re-evaluated)

| ID | Option | 2026-09-19 status |
| --- | --- | --- |
| H1 | QMP/HMP `nmi` → `ARM_CPU_SERROR` | **Still impossible** on virt TCG (all rows above). 2020 qemu-devel “Simulate NMI Injection” series did **not** land in QEMU 10.0.13. |
| H2 | Guest-only RAS / poisoned load | Still **rejected for Verified** ([ADR-044](ADR-044-taken-serror-research.md)). |
| H3 | Treat sync/BRK/FIQ / FEAT_NMI as SError | Still **forbidden**. FEAT_NMI is a GICv3 interrupt story, not async SError. |
| H5 | KVM `serror_pending` / plugin inject | Still **out of scope** for TCG virt `-kernel` smoke; this agent-box has no KVM accel. |

## Decision

1. **Hard-stop stands on the new default CPU.** Taken lower-EL SError while standing on QEMU 10 `virt` TCG + `-cpu cortex-a76` (and the documented GICv3 / `virtualization=on` / `secure=on` / `max` variants probed above) remains **deferred / non-goal** and **locked**. ADR-053 reopen gate §4 is **not** met.
2. **Park stays Verified.** Smoke must keep requiring `el0: serror-park`. Do **not** require `el0: serror`.
3. **A-clear plumbing stays dormant prep.** Keep `el0: serror-arm` / `eret_to_el0_serror` / EXPECT / QMP attempt from [ADR-045](ADR-045-taken-serror-qmp.md). Do not delete; do not claim taken.
4. **Explicit blocker (what would unlock).** Sponsor must approve **one** of:
   - **B1 — Upstream / pinned QEMU:** `virt` implements `TYPE_NMI` such that `inject-nmi` raises **async SError** (`ARM_CPU_SERROR`) on the accepted smoke CPU/GIC; re-probe dated; serial `el0: serror` under EXPECT; new ADR citing working evidence; **or**
   - **B2 — Sponsor smoke-machine ADR:** a documented, CI-runnable machine/accel/tooling change that honestly delivers async SError (not FIQ/BRK/FEAT_NMI-as-SError; not silent invent); same EXPECT + evidence ADR; **or**
   - **B3 — Permanent non-goal:** sponsor records that taken SError will not be pursued further under virt TCG (locks remain; park stays Verified).
5. **Still non-claims:** umbrella “EL0 isolated” ([ADR-055](ADR-055-el0-isolated-checklist.md)); never yank `_start`. PAN enable remains Verified separately ([ADR-080](ADR-080-pan-enable-fault.md)).

## Honesty

Say: “taken lower-EL SError stays hard-stopped on virt TCG after ADR-081 re-probe on cortex-a76; `inject-nmi` still returns `machine does not provide NMIs`; unlock needs sponsor B1/B2/B3.” Do **not** say:

- taken lower-EL SError is Verified
- “EL0 isolated” / “secure OS”
- GICv3 / `virtualization=on` / FEAT_NMI made inject work
- guest-only RAS inject is Verified
- KVM was used on this CI smoke path

## Follow-on (sponsor B1)

Sponsor chose **B1** (2026-09-19 ~08:58 CEST). Follow-on research: [ADR-082](ADR-082-b1-qemu-type-nmi.md) — **no shippable upstream/apt pin this session**; virt through QEMU master still lacks `TYPE_NMI`→async SError. Park stays Verified; taken stays blocked. Not B2/B3.

## Follow-on

[ADR-083](ADR-083-b1-qemu-nmi-pin.md) (2026-09-19): pinned QEMU opt-in Verifies taken `el0: serror`; stock park path unchanged.

## Consequences

- Docs: this ADR; [ADR-053](ADR-053-taken-serror-hard-stop.md) / [ADR-045](ADR-045-taken-serror-qmp.md) / [ADR-055](ADR-055-el0-isolated-checklist.md) / [ADR-060](ADR-060-isolation-leftovers-closure-checklist.md) / [ADR-079](ADR-079-pan-cpu-reopen.md) cross-links; honesty ledger; roadmap P-SEC-3r; threat-model **v1.56**; SUMMARY; el0 / limits / security light touch; research brief under `research/daily-briefs/`.
- Smoke / `qemu-serial-inject.py`: keep QMP attempt + park requirement; log cites ADR-081. No `src/` change required (dormant prep already present).
- Preserve all existing smoke greps. CloudAgent HELD. Do not self-merge ([ADR-002](ADR-002-pr-identity-split.md)).
