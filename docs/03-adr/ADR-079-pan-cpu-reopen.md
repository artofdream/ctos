# ADR-079 — PAN CPU reopen foundation (sponsor unlock; default `-cpu cortex-a76`)

- Status: Accepted (CPU reopen foundation). Meets [ADR-054](ADR-054-pan-enable-lock.md) reopen gate items **1–2**: sponsor-approved FEAT_PAN default probe CPU + serial `pan: present`. Enable+fault is **[ADR-080](ADR-080-pan-enable-fault.md)** (Accepted). Taken SError reopen is **ADR-081** (follow-up). Does **not** claim “EL0 isolated.”
- Date: 2026-09-19
- Tracks [#125](https://github.com/artofdream/ctos/issues/125). Sponsor unlock Sat 19 Sep 2026 via DSO. CloudAgent **HELD** — agent-box / EVO-X2 / GHA only.

## Context

[ADR-054](ADR-054-pan-enable-lock.md) locked PAN **enable** as non-goal on default `-cpu cortex-a57` (`pan: absent`, [ADR-026](ADR-026-pan-capability.md)). Reopen required: (1) sponsor-approved FEAT_PAN CPU + new ADR; (2) `ID_AA64MMFR1_EL1.PAN != 0` / `pan: present`; (3) later enable+fault ADR; (4) ledger/smoke update; keep a57 `pan: absent` historical.

Sponsor / DSO unlocked mile (1): research honest QEMU `virt` CPUs, pick **one** CI-viable default, switch smoke scripts **with this ADR cite**, print `pan: present`. Do **not** implement enable or taken SError Verified in this PR. [ADR-053](ADR-053-taken-serror-hard-stop.md) stays hard-stopped until ADR-081. Umbrella “EL0 isolated” stays non-claim ([ADR-055](ADR-055-el0-isolated-checklist.md) / [ADR-060](ADR-060-isolation-leftovers-closure-checklist.md)). Never yank `_start`. No silent `-cpu` without this ADR.

## Research (agent-box, 2026-09-19 CEST)

Host: agent-box. `qemu-system-aarch64` **10.0.13** (Debian `1:10.0.13+ds-0+deb13u1`). Machine: `-machine virt`. Short freestanding guest probe (MRS `ID_AA64MMFR1_EL1`, PL011 serial) — not a silent host invent.

| `-cpu` | `ID_AA64MMFR1_EL1.PAN` [23:20] | Serial |
| --- | --- | --- |
| `cortex-a57` | 0 | `pan: id=0` / `pan: absent` (historical) |
| `cortex-a72` | 0 | `pan: id=0` / `pan: absent` |
| `cortex-a55` | 2 | `pan: id=2` / `pan: present` |
| `cortex-a76` | 2 | `pan: id=2` / `pan: present` (**chosen default**) |
| `cortex-a710` | 2 | `pan: id=2` / `pan: present` |
| `neoverse-n1` | 2 | `pan: id=2` / `pan: present` (UDP not CI-viable here) |
| `neoverse-n2` | 2 | `pan: id=2` / `pan: present` |
| `neoverse-v1` | 2 | `pan: id=2` / `pan: present` |
| `max` | 3 | `pan: id=3` / `pan: present` (moving target — rejected as default) |

### CI / smoke viability (agent-box, same tip ELF, inject runner)

| `-cpu` | `pan:` | N1–N5 net | FAT / samples |
| --- | --- | --- | --- |
| `cortex-a57` | `absent` | ok | ok (historical) |
| `neoverse-n1` | `present` | **UDP miss** (`net: udp-tx` then `net: probe missed`) | ok after miss |
| `cortex-a76` | `present` | **ok** (udp+tcp) | ok |
| `cortex-a55` | `present` | ok | ok |
| `max` | `present` (id=3) | ok | ok (rejected as default — moving target) |

**Chosen default: `cortex-a76`.** FEAT_PAN id=2, full existing smoke (net/FAT/samples) green on agent-box QEMU 10.0.13. `neoverse-n1` is FEAT_PAN-present but **not** CI-viable for this tree’s UDP path without a separate timing/net ADR.

### `inject-nmi` note (SError mile later)

QMP `inject-nmi` on the same host still returns `machine does not provide NMIs` for `cortex-a57`, `cortex-a76`, `neoverse-n1`, and `max` (re-probed 2026-09-19). CPU switch alone does **not** unlock taken SError — that remains ADR-053 / future **ADR-081**.

## Hypotheses

| ID | Option | Notes |
| --- | --- | --- |
| **H1 (chosen)** | Default probe CPU → `-cpu cortex-a76`; smoke expects `pan: present`; cite this ADR; keep a57 rows historical; no `MSR PAN`. | FEAT_PAN id=2; full net/FAT/samples smoke green on agent-box; not `max`; `neoverse-n1` UDP-fails on this tree. |
| **H2 (rejected)** | Stay on `cortex-a57` and only document research. | Sponsor asked for concrete CPU switch evidence when a FEAT_PAN CPU works honestly. |
| **H3 (rejected)** | Default to `-cpu max`. | Field value drifts with QEMU; ADR-026 already rejected silent `max`. |
| **H4 (rejected)** | Enable PSTATE.PAN / claim Verified enable this PR. | Mile 2 = ADR-080. |
| **H5 (rejected)** | Fake `pan: present` on a57 or invent Verified SError. | Honesty fail. |

## Decision

1. **Sponsor unlock recorded.** Isolation leftovers reopen mile (1) is authorized ([#125](https://github.com/artofdream/ctos/issues/125)). ADR-054 gate items **1–2** are met by this ADR + evidence below. Items **3–4** landed in [ADR-080](ADR-080-pan-enable-fault.md).
2. **Default probe CPU.** `scripts/qemu-aarch64.sh` and `scripts/qemu-serial-inject.py` use `-cpu cortex-a76` with this ADR cite. Do not silent-edit without an ADR.
3. **Serial contract (this mile).** Guest **reads** `ID_AA64MMFR1_EL1.PAN` ([ADR-026](ADR-026-pan-capability.md) probe). On the new default: `pan: id=<n>` with `n != 0` and `pan: present`. [ADR-080](ADR-080-pan-enable-fault.md) adds `pan: enabled` / `pan: el1-fault`. Historical a57 `pan: absent` rows remain truth for a57.
4. **Probe success semantics.** `pan::observe_probe` returns success for both `absent` and `present` (ID field printed). Fail-closed only if the probe did not publish. No `MSR PAN` in this mile.
5. **Still non-claim.** Taken SError until ADR-081. Umbrella “EL0 isolated” ([ADR-055](ADR-055-el0-isolated-checklist.md) / [ADR-060](ADR-060-isolation-leftovers-closure-checklist.md)). Never yank `_start`. No Guest Linux / containers marketing.

6. **Toolchain pin.** `rust-toolchain.toml` pins `nightly-2026-09-12` (`0fc141305`) — same tip as ADR-078 ledger evidence. Floating `nightly` @ 2026-09-18 failed freestanding `cargo test` with E0463 (`can't find crate for test`). Smoke uses `cargo` (pin) not `cargo +nightly`.
7. **Follow-ups.** **ADR-080** (Accepted): enable + EL1-vs-EL0 fault. **ADR-081**: honest SError inject or machine/CPU/GIC ADR (note: `inject-nmi` still fails on virt after this CPU switch).

## Honesty

Say: “default smoke CPU is `-cpu cortex-a76` (ADR-079); `ID_AA64MMFR1_EL1.PAN != 0` → `pan: present`; enable still locked pending ADR-080; a57 `pan: absent` remains historical.” Do **not** say:

- PAN is enabled / “privileged access never”
- “EL0 isolated” / “secure OS”
- taken lower-EL SError is Verified
- the probe silently used a different `-cpu` than the scripts
- Guest Linux / containers / immutable-OS

## Consequences

- Scripts + smoke + `src/pan.rs` / `#[test_case]` updated for `cortex-a76` / `pan: present`.
- Docs: this ADR, [ADR-054](ADR-054-pan-enable-lock.md) reopen note, [ADR-060](ADR-060-isolation-leftovers-closure-checklist.md), honesty ledger, roadmap P-SEC-3k, threat-model light bump, SUMMARY, limits / el0 light touch.
- Preserve all existing smoke greps (net/FAT/samples). CloudAgent HELD. Do not self-merge ([ADR-002](ADR-002-pr-identity-split.md)).
