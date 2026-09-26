# ADR-084 brief — B2 spike on free GitHub-hosted runners (blocked)

Date: 2026-09-25 CEST (~23:45–00:30). Host: agent-box (`gh` reads) + EVO-X2 (workflow-scoped push). CloudAgent not used (over cap). No self-hosted / paid / larger runner.

## Order

Sponsor via DSO 23:44 CEST: "Go with B2 to unlock 1 and 2." Item 1 = ADR-081 **B2**: reach a real taken lower-EL SError on a **different CI machine/runner/accelerator**, distinct from the B1 pinned-QEMU opt-in job. If impossible, record honestly and stop.

## Result

- Repo is public → standard hosted runners are free.
- Temp branch `probe/adr-084-b2-kvm`, workflow `b2-serror-probe`: runs [36193423604](https://github.com/artofdream/ctos/actions/runs/36193423604) and [36193613972](https://github.com/artofdream/ctos/actions/runs/36193613972) (all 5 jobs green on run 2).
- `ubuntu-24.04-arm` / `ubuntu-22.04-arm` (Cobalt 100 / Neoverse-N2 under Hyper-V): `kvm [1]: HYP mode not available`, `/dev/kvm` absent, `-accel kvm` → `Could not access KVM kernel module`.
- `macos-15` arm64 (`Apple M1 (Virtual)`): `-accel hvf` → `HV_UNSUPPORTED`.
- `windows-11-arm`: `VMMonitorModeExtensions : False`, WHPX Disabled.
- `ubuntu-24.04` x86: `/dev/kvm` present but useless for an AArch64 guest (`invalid accelerator kvm`).
- Stock TCG on every runner: `machine does not provide NMIs`; sweep of every aarch64 machine model: `inject-nmi-ok=0`. QEMU source: no `hw/arm/*` implements `TYPE_NMI`; KVM `serror_pending` is migration-only in stock QEMU.

## Decision

B2 impossible on free hosted runners → docs-only [ADR-084](../../docs/03-adr/ADR-084-b2-free-runner-serror.md). P-SEC-3r B2 path **Planned / blocked**. B1 opt-in pin unchanged (Verified opt-in). No new Verified. Not "EL0 isolated".

## Needs sponsor

Pick one (or none): B2-S self-hosted arm64 KVM runner (security change), B2-P paid bare-metal arm64, B2-W wait for Azure arm64 nested virt (runner-images#14062). Any path also needs a small KVM VMM that injects `serror_pending` on `el0: serror-arm`.

Raw lines: [2026-09-25-adr-084-b2-probe.txt](2026-09-25-adr-084-b2-probe.txt). Workflow copy: [2026-09-25-adr-084-b2-probe-workflow.yml.txt](2026-09-25-adr-084-b2-probe-workflow.yml.txt).
