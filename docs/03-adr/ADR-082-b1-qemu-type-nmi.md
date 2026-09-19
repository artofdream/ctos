# ADR-082 — B1 follow-on: upstream/pinned QEMU `TYPE_NMI` → async SError (blocker)

- Status: Accepted (docs / research). Sponsor chose **B1** (2026-09-19 ~08:58 CEST) after [ADR-081](ADR-081-taken-serror-reopen.md). Dated upstream + agent-box research: **no shippable QEMU pin** this session makes `inject-nmi` deliver real taken SError on `virt` TCG. Taken path stays **deferred / non-goal (locked)**. `el0: serror-park` stays **Verified**. Do **not** mark `el0: serror` Verified. Not B2/B3.
- Date: 2026-09-19
- Tracks [#125](https://github.com/artofdream/ctos/issues/125). CloudAgent **HELD** — agent-box / EVO-X2 / GHA only.

## Context

[ADR-081](ADR-081-taken-serror-reopen.md) re-probed taken lower-EL SError on default `-cpu cortex-a76` and recorded unlock options B1/B2/B3. Sponsor selected **B1 — Upstream / pinned QEMU** so `virt` implements `TYPE_NMI` such that QMP `inject-nmi` / HMP `nmi` raises **async SError** (`ARM_CPU_SERROR` / taken path → serial `el0: serror` under EXPECT).

Do **not** invent FIQ / BRK / FEAT_NMI as SError. Do **not** silent machine invent. Keep `_start` at `0x4008_0000`. Umbrella “EL0 isolated” stays non-claim ([ADR-055](ADR-055-el0-isolated-checklist.md) / [ADR-060](ADR-060-isolation-leftovers-closure-checklist.md)).

## Research (agent-box, 2026-09-19 CEST)

Host: agent-box (`grok-bot-vm-401670533`). `qemu-system-aarch64` **10.0.13** (Debian `1:10.0.13+ds-0+deb13u1`). Accel: TCG only. Default smoke CPU: `-cpu cortex-a76` ([ADR-079](ADR-079-pan-cpu-reopen.md)).

### Live inject matrix (paused `-S`; QMP only)

Brief: [research/daily-briefs/2026-09-19-adr-082-b1-qemu-nmi.md](../../research/daily-briefs/2026-09-19-adr-082-b1-qemu-nmi.md). Full log: [research/daily-briefs/2026-09-19-adr-082-b1-qemu-nmi-probe-script.txt](../../research/daily-briefs/2026-09-19-adr-082-b1-qemu-nmi-probe-script.txt). Repro: `./scripts/qemu-nmi-probe.py`.

| Config | QMP `inject-nmi` |
| --- | --- |
| `virt` + `cortex-a76` (default smoke) | **error** `machine does not provide NMIs` |
| `virt` + `cortex-a57` (historical) | same |
| `virt,gic-version=3` + `cortex-a76` | same |
| `virt,gic-version=3` + `max` | same |
| `virt,virtualization=on` + `cortex-a76` | same |
| `virt,gic-version=3,virtualization=on` + `cortex-a76` | same |
| `virt,gic-version=3,secure=on` + `cortex-a76` | same |

Same failure class as ADR-081. No machine/CPU flag on this package exposes NMI.

### Upstream QEMU (source inspection, 2026-09-19)

| Tree | `hw/arm/virt.c` implements `TYPE_NMI`? | Notes |
| --- | --- | --- |
| Debian / agent-box **10.0.13** | **No** | `inject-nmi` → `machine does not provide NMIs` |
| Upstream tag **v10.0.0** | **No** | `rg TYPE_NMI` empty in `virt.c` |
| Upstream tag **v10.1.0** | **No** | same |
| Upstream **master** (fetched 2026-09-19) | **No** | FEAT_NMI / GICv3 `has-nmi` wires `ARM_CPU_NMI` (architectural **interrupt** NMI), **not** async SError, and does **not** register `TYPE_NMI` for QMP `inject-nmi` |

Historical series that would have satisfied B1:

- 2019–2020 qemu-devel **“Simulate NMI Injection”** / **“Support SError injection”** (Gavin Shan et al.): proposed `TYPE_NMI` on virt → raise `ARM_CPU_SERROR`. **Never merged** into virt.
- 2024 FEAT_NMI / FEAT_GICv3_NMI series: different architecture (PPI/SPI NMI interrupts + `PSTATE.ALLINT`). **Not** a substitute for async SError Verified.
- 2026-08 qemu-devel `nmi_inject` API cleanup: still “machine does not provide NMIs” when no `TYPE_NMI` object exists.

`CPU_INTERRUPT_VSERR` / `EXCP_VSERR` in current `target/arm` is the **virtual** SError (EL2→EL1) path — not a host QMP inject into our EL1/EL0 standing TCG guest.

### CI pin options evaluated

| Option | CI-runnable? | Delivers `el0: serror`? | 2026-09-19 verdict |
| --- | --- | --- | --- |
| Apt pin newer Debian qemu | Yes (GHA `apt-get`) | **No** — 10.0.x / published 10.1 packages still lack virt `TYPE_NMI`→SError | Rejected as unlock |
| Build upstream `master` from source + cache | Possible (slow; multi-min) | **No** — master also lacks the wiring | Rejected as unlock |
| Local QEMU patch (draft under `research/qemu-nmi/`) + build-from-source | Possible next session | **Would**, if patch verified under EXPECT | **Not shipped this session** — needs sponsor-reviewed patch + reproducible Dockerfile/GHA cache before smoke can require `el0: serror` |
| Machine flag / `-cpu *,nmi=on` alone | N/A | **No** — `Property 'max-arm-cpu.nmi' not found` on 10.0.13; FEAT_NMI ≠ SError | Forbidden as Verified |

Draft local-patch sketch (research only, **not** CI-wired): [research/qemu-nmi/README.md](../../research/qemu-nmi/README.md) + [research/qemu-nmi/0001-hw-arm-virt-TYPE_NMI-raise-SError.patch](../../research/qemu-nmi/0001-hw-arm-virt-TYPE_NMI-raise-SError.patch).

## Decision

1. **B1 chosen; B1 not met this session.** Sponsor selected B1. Upstream and packaged QEMU (through **master** as of 2026-09-19) do **not** implement virt `TYPE_NMI` → async SError. No apt/git pin alone unlocks taken SError.
2. **Blocker (explicit).** Unlock remains: wait for **upstream** virt `TYPE_NMI`→`ARM_CPU_SERROR` (or equivalent honest QMP), **or** a **sponsor-approved pinned QEMU build** (Dockerfile / GHA cache) that carries a reviewed local patch proving `inject-nmi` → serial `el0: serror` under EXPECT on agent-box + GHA. Until then, hard-stop from ADR-053/081 stands.
3. **Park stays Verified.** Smoke keeps requiring `el0: serror-park`. Do **not** require `el0: serror`.
4. **A-clear plumbing stays dormant prep.** Keep ADR-045 EXPECT / `el0: serror-arm` / QMP attempt.
5. **Not B2/B3.** This ADR does not approve a different smoke machine (B2) and does not declare permanent non-goal (B3).
6. **Still non-claims:** umbrella “EL0 isolated”; never yank `_start`; FEAT_NMI / FIQ / BRK are not SError.

## Honesty

Say: “sponsor chose B1; ADR-082 research finds no upstream/packaged QEMU pin that makes `inject-nmi` deliver taken SError on virt TCG; park stays Verified; taken stays blocked pending upstream or a reviewed pinned build.” Do **not** say:

- taken lower-EL SError is Verified / `el0: serror` is required
- a newer apt qemu “should” unlock without probe
- FEAT_NMI / GICv3 NMI is the taken-SError path
- the research patch is CI-wired or proven
- “EL0 isolated” / “secure OS”

## Consequences

- Docs: this ADR; cross-links on ADR-053/081/045/055/060; honesty ledger; roadmap P-SEC-3r; threat-model **v1.57**; SUMMARY; el0 / limits / security light touch; research brief + probe script + draft patch under `research/qemu-nmi/`.
- Smoke: unchanged fail-closed park requirement. No `src/` change.
- **Follow-on:** [ADR-083](ADR-083-b1-qemu-nmi-pin.md) ships the pinned build + opt-in `el0: serror` Verified (2026-09-19).
- CloudAgent HELD. Do not self-merge ([ADR-002](ADR-002-pr-identity-split.md)).
