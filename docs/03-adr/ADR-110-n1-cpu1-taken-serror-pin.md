# ADR-110 — N1: taken SError on core 1 at EL0 (pinned QEMU, `-smp 2`, cpu-index=1)

- Status: Accepted (implementation + evidence, opt-in). Sponsor go relayed by DSO on 2026-10-23 at 20:17 CEST (N1 from [ADR-108](ADR-108-taken-serror-qemu-spike.md)).
- Date: 2026-10-23
- Base: main `c420fba`. ADR-107 (#163, FAT32), ADR-108 (#165, the spike) and ADR-109 (#164, inbound TCP) are taken, so this is ADR-110.
- Machine: amd64 workstation (TCG; builds in an `ubuntu:24.04` container). CI: the `QEMU NMI pin smoke (taken SError)` job.

## Context

The B1 pinned QEMU (v10.0.0 + patch 0001, [ADR-083](ADR-083-b1-qemu-nmi-pin.md)) gave a taken lower-EL SError only on core 0, at `-smp 1`. On core 1 (S4/S5, [ADR-104](ADR-104-smp-s4-percpu-asid-generation-el0-cpu1.md)/[ADR-105](ADR-105-smp-s5-concurrent-el0-runq.md)) there was no SError probe. The ADR-109 spike showed that the pinned build's HMP `nmi` honours `cpu-index`.

## Decision

1. **Opt-in features:**
   - `smp2-serror-probe` (implies `smp2-probe`): after core 1's 13 S4 probes, core 1 prints `smp: cpu1 serror-arm masked=no cpu=1` and ERETs to the same standing-EL0 spin as the single-core probe, with PSTATE.A clear and the banked per-core EXPECT flags.
   - `smp2-serror-masked` is the negative build: it ERETs with A set.
2. **Handler (fail-closed):** in that build, core 1's lower-EL SError handler prints its own locked line, `smp: cpu1 el0 serror taken cpu=1`. It never prints the core-0 `el0: serror` cue. The probe prints `smp: cpu1 serror taken=yes …` only if the handler ran **and** the standing trip restored. Otherwise it prints `smp: cpu1 serror park taken=no …`.
3. **Host:** `qemu-serial-inject.py`, on the `smp: cpu1 serror-arm` cue and only when `CTOS_QEMU_NMI_CPU1=<n>` is set, does QMP `stop` → `human-monitor-command nmi` with `cpu-index=<n>` → `cont`. QMP `inject-nmi` has no CPU argument.
4. **Check:** `scripts/smp2-serror-pin.sh`. Every row requires the host `nmi=>ok`, `smp: ok cpu1 s4`, and no unhandled exception.

   | Row | Build | nmi to | Must see | Must not see |
   | --- | --- | --- | --- | --- |
   | P | probe | cpu-index=1 | `cpu1 el0 serror taken` and `taken=yes` | `park` |
   | N0 | probe | cpu-index=0 | `park taken=no masked=no` | `cpu1 el0 serror taken` |
   | NM | masked | cpu-index=1 | `park taken=no masked=yes` | `cpu1 el0 serror taken` |

5. **CI:** a new step in `qemu-nmi-pin` after the unchanged single-core step (`CTOS_REQUIRE_TAKEN_SERROR=1`).

## Evidence (2026-10-23, amd64 workstation, pinned QEMU 10.0.0)

- `smp2-serror-pin.sh`: P ok (`nmi(cpu-index=1)=>ok`, `smp: cpu1 el0 serror taken cpu=1`), N0 ok, NM ok.
- The full stock smoke (`CTOS_SMP2_SMOKE=1`) and the single-core pin smoke (`CTOS_REQUIRE_TAKEN_SERROR=1`) still pass.
- CI results are on the PR.

## Honesty

Say: "With the repo's pinned QEMU, a host-injected SError aimed at core 1 (`cpu-index=1`) is taken by ctos core 1 while it stands at EL0, on `-smp 2` TCG. Aimed at core 0, or with core 1's A bit set, it is not taken."

Do **not** say:
- that stock QEMU does this;
- that it is hardware RAS;
- that the "EL0 isolated" sentence changes. A claim change needs a separate sponsor accept.

Also not shown:
- whether core 0 takes the N0 SError later: it stays pending with A masked, and nothing after unmasks it;
- concurrency with S5 (core 1 runs the probe before S5);
- KVM.

## Consequences

- `Cargo.toml` gains two features.
- Code: `src/el0.rs` `cpu1_serror_probe`, a cfg branch in `src/exception.rs` for the handler, one cfg call in `src/smp.rs`, and `smp::line` becomes `pub(crate)`.
- Scripts: `scripts/qemu-serial-inject.py` (`hmp_nmi`, env-gated) and `scripts/smp2-serror-pin.sh`.
- Workflow: one step added to `qemu-nmi-pin`.
- Docs: ledger row, SUMMARY, moc, brief.
- The default build, default smoke and `_start` are unchanged.
