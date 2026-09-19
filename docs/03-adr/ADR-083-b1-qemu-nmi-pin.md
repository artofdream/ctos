# ADR-083 — B1 pinned QEMU: `inject-nmi` → taken `el0: serror` (Verified opt-in)

- Status: Accepted (implementation + evidence). Sponsor B1 (ADR-081/082) met via **pinned QEMU build** (not upstream merge). Dated proof: QMP `inject-nmi` succeeds and guest serial shows **`el0: serror`** under EXPECT. Stock distro QEMU path **unchanged** (park Verified). Umbrella “EL0 isolated” stays **non-claim**.
- Date: 2026-09-19
- Tracks [#125](https://github.com/artofdream/ctos/issues/125). CloudAgent **HELD** — agent-box proof; EVO-X2 preferred for heavy build but offline this session (`f074e48c-…` intent). Do not self-merge ([ADR-002](ADR-002-pr-identity-split.md)).

## Context

[ADR-082](ADR-082-b1-qemu-type-nmi.md) found no upstream/packaged QEMU pin: virt through **master** lacks `TYPE_NMI`→async SError. DSO (2026-09-19) preferred a **pinned custom QEMU** over waiting upstream. Draft sketch lived under `research/qemu-nmi/`.

## Decision

1. **Pin QEMU v10.0.0** (`7c949c53e936aa3a658d84ab53bae5cadaa5d59c`) + reviewed local patch [`research/qemu-nmi/0001-hw-arm-virt-TYPE_NMI-raise-SError.patch`](../../research/qemu-nmi/0001-hw-arm-virt-TYPE_NMI-raise-SError.patch).
2. **Patch semantics (honest):** add TCG `EXCP_SERROR` / `CPU_INTERRUPT_SERROR`; virt implements `TYPE_NMI` so `inject-nmi` raises that line. Masked by `PSTATE.A`. **Not** FEAT_NMI / FIQ / BRK.
3. **Install prefix:** `tools/qemu-nmi/` via `./scripts/build-qemu-nmi.sh` (bin/share gitignored).
4. **Opt-in smoke:** `CTOS_QEMU=…/qemu-system-aarch64` + `CTOS_REQUIRE_TAKEN_SERROR=1` fail-closed requires bare `el0: serror`. Default (unset) still requires `el0: serror-park`.
5. **Host inject timing:** `qemu-serial-inject.py` issues QMP `stop` → `inject-nmi` → `cont` on `el0: serror-arm` so TCG cannot race past the A-clear window.
6. **CI/Docker:** default stays distro QEMU. Dockerfile `ARG CTOS_BUILD_QEMU_NMI=0` opt-in. GHA job snippet in [docs/dev/qemu-nmi-pin-ci.md](../dev/qemu-nmi-pin-ci.md) (OAuth push lacks `workflow` scope this session — paste when available). Do not force multi-minute QEMU compile on every PR.
7. **Still non-claims:** umbrella EL0 isolated; never yank `_start`; park remains Verified on stock QEMU.

## Evidence (2026-09-19 CEST, agent-box)

| Probe | Result |
| --- | --- |
| `CTOS_QEMU=tools/qemu-nmi/bin/… ./scripts/qemu-nmi-probe.py` | `inject-nmi` **ok** on virt + cortex-a76 / a57 / GICv3 / virt-on / secure / max |
| Stock `qemu-system-aarch64` 10.0.13 probe | still `machine does not provide NMIs` |
| `CTOS_QEMU=… CTOS_REQUIRE_TAKEN_SERROR=1 ./scripts/qemu-smoke.sh` | **`el0: serror`** present; `qemu-smoke: ok` |
| Stock inject (no pin) | `inject-nmi` error + `el0: serror-park` |

Machine tag: **(agent-box)**; **(evo-x2)** intent only (CloudAgent HELD / cts-ai offline).

## Honesty

Say: “B1 met with a pinned QEMU build; taken `el0: serror` Verified under opt-in `CTOS_QEMU` + `CTOS_REQUIRE_TAKEN_SERROR=1`; stock QEMU still parks.” Do **not** say taken is Verified on distro QEMU, or that FEAT_NMI is the path, or “EL0 isolated.”

## Consequences

- Docs: this ADR; ADR-082/081/053/045 cross-links; honesty ledger; SUMMARY; threat-model bump; `research/qemu-nmi/` + `tools/qemu-nmi/PIN.txt`.
- Smoke/scripts: `CTOS_QEMU` honored; taken gate opt-in; stock park preserved.
- Dockerfile: `CTOS_BUILD_QEMU_NMI` ARG. GHA: see docs/dev/qemu-nmi-pin-ci.md (workflow file unchanged this PR).
