# ADR-090 — Taken-SError evidence class for the umbrella = B1 pin + B2-P (M4, sponsor D1)

- Status: Accepted (docs + CI workflow). Records sponsor decision **D1** (DSO relay 2026-09-26 ~14:35 CEST). The taken lower-EL SError evidence that counts for the umbrella is **the B1 pinned-QEMU CI job plus the B2-P run**, stated verbatim below and copied into the draft accept ADR (ADR-091). Makes the `qemu-nmi-pin` job **unconditional**. No `src/` logic change; one stale M2 doc comment in `src/teardown.rs` is fixed (the allowlist has been stub-only since ADR-088). The umbrella “EL0 isolated” stays **Planned / non-claim**: the sponsor's written accept (B3) is still absent.
- Date: 2026-09-26
- Base: stacked on PR [#141](https://github.com/artofdream/ctos/pull/141) (ADR-089, M3).

## Context

ADR-055 §B row 1 required “honest host inject that delivers async SError on the **accepted** smoke machine” (ADR-060). [ADR-086](ADR-086-el0-isolated-checklist-audit.md) found two taken-SError sources, neither on that machine as worded. **B1** ([ADR-083](ADR-083-b1-qemu-nmi-pin.md)) is a CI job on a locally patched QEMU, gated by repo variable `CTOS_BUILD_QEMU_NMI=1`, so it is opt-in. **B2-P** ([ADR-085](ADR-085-b2p-graviton-kvm-serror.md)) is one paid real-hardware KVM run, not CI. ADR-086 asked D1: which evidence counts? The sponsor accepted the recommendation (i)+(ii): both together, stated verbatim, with no silent redefinition.

## Decision

1. **D1 evidence class (verbatim; ADR-091 must quote this paragraph unchanged):**

   > *Taken lower-EL SError while standing at EL0 is evidenced by two sources together. (i) **B1:** the GitHub Actions job `qemu-nmi-pin` (“QEMU NMI pin smoke (taken SError)”) in `.github/workflows/smoke.yml`. It builds QEMU v10.0.0 (`7c949c53e936aa3a658d84ab53bae5cadaa5d59c`) with reviewed local patch `research/qemu-nmi/0001-hw-arm-virt-TYPE_NMI-raise-SError.patch` (TCG: `virt` `TYPE_NMI` → async SError, masked by `PSTATE.A`), boots the default image on `virt -cpu cortex-a76` TCG, issues QMP `stop` → `inject-nmi` → `cont` on the guest's `el0: serror-arm` cue, and with `CTOS_REQUIRE_TAKEN_SERROR=1` fails closed unless the bare `el0: serror` line (printed only by the lower-EL SError vector while the standing-EL0 expectation is armed) appears. It runs on every push and pull request. (ii) **B2-P:** ADR-085 run 4, 2026-09-26 13:26–13:33 CEST, AWS Graviton3 `c7g.metal` (eu-north-1b, Spot), ctos commit `159b178`, QEMU `-accel kvm` with patch 0002 (`inject-nmi` → `KVM_SET_VCPU_EVENTS{serror_pending=1}` → `HCR_EL2.VSE`), opt-in `b2-serror` profile: `el0: serror` + `b2: taken`, fail-closed `b2-kvm-smoke.py` PASS. It is a one-off real-hardware run, not CI. Stock distro QEMU (the default smoke) cannot inject and continues to require `el0: serror-park`.*

2. **Make B1 unconditional.** Remove `if: ${{ vars.CTOS_BUILD_QEMU_NMI == '1' }}` from `qemu-nmi-pin` and rename the job from “(opt-in)” to “QEMU NMI pin smoke (taken SError)”. Why: D5 makes the smoke a GitHub-**required** check before the flip. GitHub treats a job skipped by `if:` as passing a required check, so a variable-gated job would let a PR pass without taking an SError. This reverses [ADR-083](ADR-083-b1-qemu-nmi-pin.md) §6 (“do not force multi-minute QEMU compile on every PR”) on purpose. Cost: ≈ 10–15 min on a free GitHub-hosted `ubuntu-24.04` runner per push/PR. The repo is public, so standard runners are free. The repo variable becomes unused and can be deleted by the sponsor at any time; its value no longer matters.
3. **Reworded requirement** (ADR-055 §B row 1, ADR-060 taken-SError row): *“Taken lower-EL SError while standing, on the D1 evidence class (ADR-090): B1 `qemu-nmi-pin` green, unconditional on every push/PR, plus the B2-P one-off KVM run (ADR-085 run 4).”*
4. **Row status:** reworded row 1 = **Met (D1 evidence class)**. The B1 leg is green after the boot-path changes of this session: #139 (post-M1) pin jobs 108406107228 / 108406127586 and #140 (post-M2) pin job 108408283187 (`inject-nmi=>ok` → bare `el0: serror`, 2026-09-26 14:56–15:10 CEST). The first **unconditional** run is on this PR's CI. This is **not** the umbrella: row 3 (sponsor accept) remains, see M5 → ADR-091.
5. **Not changed:** the default smoke on stock QEMU still requires `el0: serror-park` (ADR-053/081). ADR-053's hard-stop stays true **for stock virt TCG**. Dockerfile `ARG CTOS_BUILD_QEMU_NMI=0` stays opt-in (local images only). No KVM in CI.

## Caveats stated with the evidence (must travel with any quote of §1)

- **B1 is a locally patched emulator.** The SError is raised by QEMU TCG code this repo carries (patch 0001), not by stock upstream QEMU and not by hardware. It proves that the guest's lower-EL SError path takes the exception; it does not prove any CPU's RAS behaviour.
- **B2-P is historical.** It ran once at `159b178`, before this session's M1/M2 boot-path changes (identity leftover tears, MMIO high alias). The `b2-serror` profile still TCG-boots through its markers after M2 (agent box, ~14:44 CEST), but it was **not** re-run under KVM. A re-verify is one more paid `c7g.metal` Spot run, ≈ **$0.2** (ADR-085 cost basis). D1 does not require it; it is a sponsor option.
- **B2-P used a reduced profile** (no GIC / timer / virtio / FAT), per ADR-085.
- Neither leg is real hardware running the full default image.

## Honesty

Say: “per sponsor D1 (ADR-090), taken lower-EL SError is evidenced by the B1 pinned-QEMU CI job, which is unconditional and fail-closed on every push/PR, plus the one-off B2-P Graviton3 KVM run (ADR-085 run 4). Stock QEMU parks.” Do **not** say “taken SError on stock QEMU”, “taken SError Verified on real hardware in CI”, “SError hard-stop lifted” (it stands for stock TCG), or “EL0 isolated”.

## Consequences

- `.github/workflows/smoke.yml`: `qemu-nmi-pin` unconditional and renamed. [docs/dev/qemu-nmi-pin-ci.md](../dev/qemu-nmi-pin-ci.md) updated.
- **D5 / M6 required-check names** (branch protection is **not** changed here; the sponsor/DSO sets it at M6): `QEMU aarch64 smoke (ubuntu-24.04-arm)`, `QEMU aarch64 smoke (ubuntu-24.04)`, `QEMU NMI pin smoke (taken SError)`.
- ADR-055 §B row 1, ADR-060 and ADR-083 follow-ons; ledger, roadmap P-SEC-3r/3l, threat model **v1.65**, [el0.md](../framework/el0.md), SUMMARY, MOC.
- Next: M5 (ADR-091 draft accept). Stop before M6.
