# ADR-111 — N2: EL2 shim → taken **virtual SError** at EL0 on stock QEMU (opt-in)

- Status: Accepted (implementation + evidence, opt-in). Sponsor go relayed by DSO on 2026-10-23 at 20:17 CEST (N2 from [ADR-109](ADR-109-taken-serror-qemu-spike.md)).
- Date: 2026-10-23
- Base: main `c420fba`. ADR-107..110 are held by open PRs #163, #164, #165 and the N1 PR, so this is ADR-111.
- Machine: amd64 workstation (TCG; builds in an `ubuntu:24.04` container).

## Context

[ADR-109](ADR-109-taken-serror-qemu-spike.md) found that stock QEMU has **no physical SError source**, but it does model the architectural **virtual SError** (`HCR_EL2.VSE`) for a guest that owns EL2. ctos runs at EL1 and `_start` (0x4008_0000) must not change. So a tiny EL2 shim must sit in front of it.

## Decision

1. **`tools/el2-shim`:** a stand-alone ELF linked at **PA 0**, the virt flash window, away from RAM and the ctos image. It is loaded with `-device loader,file=<shim>,cpu-num=0`, which also points core 0's reset PC at it; QEMU still loads the ctos ELF with `-kernel`.
   - At EL2 it sets `HCR_EL2 = RW | AMO`. IRQ/FIQ stay at EL1.
   - It also sets `CNTHCTL_EL2 = 3`, `CNTVOFF_EL2 = 0`, `CPTR_EL2 = 0x33ff`, `MDCR_EL2 = 0`, and VPIDR/VMPIDR mirroring MIDR/MPIDR.
   - It prints `el2-shim: el1` and ERETs to 0x4008_0000 at EL1h with DAIF masked and x0..x3 as QEMU set them. SCTLR_EL1 keeps its reset value.
   - It uses no RAM; its only scratch is `TPIDR_EL2`.
   - **HVC ABI** (`hvc #0xC7`, only x0 changes): 0 = query (returns magic `0x5E2E1D00`), 1 = set `HCR_EL2.VSE`, 2 = clear it. Any other HVC (ctos PSCI `hvc #0`) is forwarded as `smc #0` to QEMU's PSCI. Any other EL2 exception prints `el2-shim: unexpected` and parks.
2. **ctos feature `el2-vserror`** (opt-in). In the standing-EL0 SError window, after `el0: serror-arm`:
   - It checks `ID_AA64PFR0_EL1.EL2`, and only then queries the shim. With no EL2 it issues no HVC.
   - It asks for VSE, prints `el0: virtual serror ask el2-shim=… vse=… masked=…`, and ERETs with A clear.
   - The handler counts the SError only if ESR EC is 0x2f, and prints `el0: virtual serror taken (el2 vse) ec=0x2f esr=…`. The default `el0: serror` cue is never printed in this build.
   - VSE is cleared after the trip.
   - `el2-vserror-masked` is the negative build: it ERETs with A set.
3. **`qemu-serial-inject.py`:** opt-in `CTOS_QEMU_MACHINE` and `CTOS_QEMU_EXTRA`. When they are unset the argv is unchanged.
4. **Check:** `scripts/el2-vserror-smoke.sh` (fail-closed). Every row must reach `el0: ok` with no unhandled exception.

   | Row | Build | QEMU | Must see | Must not see |
   | --- | --- | --- | --- | --- |
   | P | `el2-vserror` | `virt,virtualization=on` + shim | `ask el2-shim=yes vse=set masked=no`, `virtual serror taken (el2 vse) ec=0x2f` | `serror-park` |
   | NA | default | same + shim | `el2-shim: el1`, `serror-park` | `virtual serror` |
   | NM | `el2-vserror-masked` | same + shim | `vse=set masked=yes`, `serror-park` | taken |
   | NS | `el2-vserror` | plain `virt` (no EL2), no shim | `el2-shim=no vse=no`, `serror-park` | taken, `el2-shim: el1` |

5. **CI:** a new opt-in job, `QEMU EL2 shim virtual SError (opt-in, ADR-111; not required)`, on stock Ubuntu 24.04 QEMU.
   - The default build, `qemu-smoke.sh` and the required checks are unchanged.
   - The NA row also shows the default image runs fully under the shim.

## Evidence (2026-10-23, amd64 workstation)

- Stock QEMU 8.2.2 (Ubuntu 24.04, container): P, NA, NM and NS all ok.
- Stock QEMU 10.2.1 (host distro): P, NA, NM and NS all ok. P line: `el0: virtual serror taken (el2 vse) ec=0x2f esr=0xbe000000`.
- CI results are on the PR.

## Honesty

Say: "On stock QEMU (`-M virt,virtualization=on`) with the repo's EL2 shim, the full ctos image takes a **virtual SError** (EL2-injected via `HCR_EL2.VSE`) at EL0, and it is not taken without the ask, with A masked, or without EL2."

Do **not** say:
- that this is a physical SError or hardware RAS;
- that the default boot has an EL2;
- that it covers cpu1 (`-smp 1` only; the shim does not wrap PSCI `CPU_ON`);
- that the "EL0 isolated" sentence changes. A claim change needs a separate sponsor accept.

## Consequences

- New: `tools/el2-shim/` and `src/el2.rs` (cfg).
- Changed, all opt-in: cfg branches in `src/el0.rs` and `src/exception.rs`, and two features in `Cargo.toml`.
- Scripts: env-gated `qemu-serial-inject.py`, plus `scripts/el2-vserror-smoke.sh`.
- Workflow: one new opt-in job.
- Docs: ledger row, SUMMARY, moc, brief.
- `_start`, the default build and the default smoke are unchanged.
