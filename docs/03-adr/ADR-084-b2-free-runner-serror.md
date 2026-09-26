# ADR-084 — B2 spike: taken SError on a different free GitHub-hosted runner (blocked — no arm64 KVM/HVF/WHPX)

- Status: Accepted (docs / evidence). Sponsor chose **B2** ("Go with B2 to unlock 1 and 2", DSO relay 2026-09-25 23:44 CEST). ADR-081 **B2** spike on free GitHub-hosted runners: **impossible this session**. No free hosted runner exposes a hardware accelerator that can run an AArch64 guest (`/dev/kvm` absent on arm64 Linux, HVF `HV_UNSUPPORTED` on macOS arm64, no VT/WHPX on Windows arm64). Stock QEMU TCG on those runners still returns `machine does not provide NMIs`. The B2 (non-B1) path for P-SEC-3r stays **Planned / blocked** pending a sponsor decision. **No** new Verified. B1 opt-in pin ([ADR-083](ADR-083-b1-qemu-nmi-pin.md)) is unchanged and is **not** B2.
- Date: 2026-09-25
- Tracks P-SEC-3r (taken lower-EL SError). Builds on [ADR-081](ADR-081-taken-serror-reopen.md) §4 B2, [ADR-053](ADR-053-taken-serror-hard-stop.md) reopen gate, [ADR-083](ADR-083-b1-qemu-nmi-pin.md). CloudAgent **not used** (over cap until 2026-10-04). No self-hosted runner registered. No paid / larger runner used. Do not self-merge ([ADR-002](ADR-002-pr-identity-split.md)).

## Context

ADR-081 §4 listed three unlocks for a taken lower-EL SError while standing: **B1** pinned/upstream QEMU with `TYPE_NMI`→async SError, **B2** a documented CI-runnable *machine / accel / tooling* change that honestly delivers async SError, **B3** permanent non-goal. B1 landed as an **opt-in** pinned-QEMU job (`qemu-nmi-pin`, repo variable `CTOS_BUILD_QEMU_NMI=1`, ~12–13 min; ADR-082/083, PRs #129–#132). That is still stock-`virt` **TCG** with a *patched emulator*.

The sponsor asked for B2 as a **genuinely different path**: run on a different CI machine/runner/accelerator (for example real arm64 hardware + KVM) and reach a real taken SError there — not a re-run of the B1 pin. Hard rules for this spike: free runners only (flag before paid/larger runners), **no** self-hosted runner registration, **no** CloudAgents, and if B2 is impossible, record it with evidence and stop (no slide into B1/B3 variants).

## B2 hypotheses (what a genuine B2 would need)

| ID | Candidate SError source | Needs | Notes |
| --- | --- | --- | --- |
| K1 | KVM `KVM_SET_VCPU_EVENTS` with `exception.serror_pending=1` (KVM sets `HCR_EL2.VSE` → guest takes a **virtual SError**, masked by `PSTATE.A`) | arm64 host with `/dev/kvm` + a small VMM (or QEMU change) that injects on the `el0: serror-arm` cue | Stock QEMU only get/puts `env->serror.pending` for migration (`target/arm/kvm.c`, `target/arm/machine.c`); **no** QMP/HMP setter. So K1 is a custom VMM, not stock QEMU. |
| K2 | QEMU `inject-nmi` under `-accel kvm` | same host | `inject-nmi` is machine-level (`hw/core/nmi.c` walks for a `TYPE_NMI` object). No `hw/arm/*` implements `TYPE_NMI` on QEMU master (code search 2026-09-25: only hppa / i386 / m68k / ppc / s390x / xtensa / macio). So K2 fails regardless of accelerator. |
| K3 | Guest access to an unbacked / external-abort region reported asynchronously | real hardware or KVM | Under KVM, unbacked IPA → MMIO exit to the VMM; QEMU can only inject a **synchronous** external data abort (`KVM_CAP_ARM_INJECT_EXT_DABT`). Async reporting is implementation-defined and not reproducible on a CI VM. Also a guest-only source (ADR-044 H2 rejected for Verified). |
| H1' | Stock TCG `inject-nmi` on a *different* runner (native arm64) | any runner | Same QEMU code as the agent box → expected `machine does not provide NMIs`. Control row. |

K1 is the only honest B2 SError source, and it requires an arm64 host that exposes a hypervisor to the job. So the spike's first question is: **does any free GitHub-hosted runner expose one?**

## Probe (GitHub Actions, 2026-09-25 CEST)

Repo `artofdream/ctos` is **public**. Standard GitHub-hosted runners (including `ubuntu-24.04-arm`, `ubuntu-22.04-arm`, `macos-15`, `windows-11-arm`) are **free** for public repos under GitHub's Actions billing policy. No larger runner labels used. Temporary branch `probe/adr-084-b2-kvm` (workflow `b2-serror-probe`, research only, not a gate; copy kept at [research/daily-briefs/2026-09-25-adr-084-b2-probe-workflow.yml.txt](../../research/daily-briefs/2026-09-25-adr-084-b2-probe-workflow.yml.txt)).

- Run 1 (`c198bf1`, 23:47 CEST): [36193423604](https://github.com/artofdream/ctos/actions/runs/36193423604) — facts + KVM/HVF/Hyper-V checks (22.04-arm job failed only because the ADR-082 probe script races QEMU 6.2, which lacks `cortex-a76`; not a finding).
- Run 2 (`749cd32`, 23:49 CEST): [36193613972](https://github.com/artofdream/ctos/actions/runs/36193613972) — all 5 jobs green; adds an all-machine `TYPE_NMI` sweep and an unconditional HVF boot attempt.

| Runner (image) | Host | Accelerator result (exact log line) | Stock TCG `inject-nmi` |
| --- | --- | --- | --- |
| `ubuntu-24.04-arm` (`20260920.129.1`) [job](https://github.com/artofdream/ctos/actions/runs/36193613972/job/108264281869) | Azure Hyper-V VM, `Neoverse-N2` (Cobalt 100), kernel `6.17.0-1022-azure` | dmesg `kvm [1]: HYP mode not available`; `b2-probe: /dev/kvm=absent`; QEMU 8.2.2 `-accel kvm`: `Could not access KVM kernel module: No such file or directory` / `failed to initialize kvm` | `virt` × a76/a57/GICv3/max/virt-on/secure: all `machine does not provide NMIs`; sweep `machines-probed=118 inject-nmi-ok=0` |
| `ubuntu-22.04-arm` (`20260920.137.1`) [job](https://github.com/artofdream/ctos/actions/runs/36193613972/job/108264281751) | same class, kernel `6.8.0-1064-azure` | `kvm [1]: HYP mode not available`; `/dev/kvm=absent`; QEMU 6.2 `-accel kvm` same failure | sweep `machines-probed=101 inject-nmi-ok=0` (QEMU 6.2 wording `this feature or command is not currently supported`) |
| `macos-15` arm64 (`20260907.0337.1`) [job](https://github.com/artofdream/ctos/actions/runs/36193613972/job/108264281730) | `Apple M1 (Virtual)`, Darwin 24.6.0 `RELEASE_ARM64_VMAPPLE` | `sysctl: unknown oid 'kern.hv_support'`; `kern.hv_vmm_present: 1`; QEMU 11.1.1 `-accel hvf`: `Error: ret = HV_UNSUPPORTED (0xfae9400f, at ../target/arm/hvf/hvf.c:1269)` | n/a (HVF has no SError inject API anyway) |
| `windows-11-arm` (`20260920.164.1`) [job](https://github.com/artofdream/ctos/actions/runs/36193613972/job/108264281535) | `Cobalt 100` Hyper-V guest | `VirtualizationFirmwareEnabled : False`, `VMMonitorModeExtensions : False`, `HypervisorPlatform (WHPX) state = Disabled` (enabling needs a reboot; no nested virt) | n/a |
| `ubuntu-24.04` x86_64 (control) [job](https://github.com/artofdream/ctos/actions/runs/36193613972/job/108264281758) | AMD EPYC (7763 run 1, 9V74 run 2), Hyper-V | `/dev/kvm=present` (`kvm_amd`) but `qemu-system-aarch64 -accel kvm`: `invalid accelerator kvm` (x86 KVM cannot run an AArch64 guest) | same `machine does not provide NMIs`; sweep `machines-probed=117 inject-nmi-ok=0` |

Sweep note: on `ubuntu-24.04-arm`, 92 machine models answered `machine does not provide NMIs` and 26 did not start without board-specific `-cpu`/firmware (for example `sbsa-ref`, `xlnx-zcu102`); the QEMU source search above shows none of them implements `TYPE_NMI` either.

Consistent with public upstream reports: [actions/runner-images#14062](https://github.com/actions/runner-images/issues/14062) ("KVM on ARM runners" — Azure arm64 VMs lack nested virtualization).

## Decision

1. **B2 on free GitHub-hosted runners is impossible (2026-09-25).** No free hosted runner can run ctos under a hardware accelerator (KVM/HVF/WHPX) on AArch64, so the only honest B2 SError source (K1, KVM virtual SError) cannot be reached. TCG on a different runner is the same emulator code and still has no NMI/SError path (H1' control).
2. **Stop here.** No B2 CI job, no new smoke marker, no `src/` change. Do **not** slide into a B1 variant (patched QEMU on the arm runner) or B3. Do **not** relabel the B1 pin as B2.
3. **Status stays honest.**
   - Stock QEMU TCG: taken SError **deferred / non-goal (hard-stopped)**; smoke keeps requiring `el0: serror-park` (ADR-053/081).
   - B1 opt-in pin: taken `el0: serror` **Verified (opt-in)** only in `qemu-nmi-pin` (ADR-083). Unchanged.
   - **B2 (different runner/accelerator): Planned / blocked** — probe ran and lost on free runners (ledger row **Failed** for the free-runner probe).
4. **What would unlock B2 (sponsor decision required; none taken here):**
   - **B2-S — self-hosted arm64 runner with KVM** on sponsor hardware (bare-metal arm64 Linux booted at EL2, e.g. an Ampere/Graviton-metal box, Raspberry Pi 5, or an Apple-silicon machine running Linux natively). Security change: registering a self-hosted runner on a public repo needs explicit sponsor go (fork PRs must not reach it). Not done.
   - **B2-P — paid bare-metal arm64** (cloud `*.metal` or a third-party runner vendor that exposes arm64 KVM). Costs money; flagged, not used. (WarpBuild documents nested virt on x64 only.)
   - **B2-W — wait** for GitHub/Azure to offer nested virtualization on arm64 hosted runners (runner-images#14062), then re-run this probe (`workflow_dispatch` copy in the research brief).
   Any of those still needs **K1 tooling**: a small KVM VMM (or reviewed QEMU change) that loads the ctos ELF at `0x4008_0000`, provides PL011 + GIC, and calls `KVM_SET_VCPU_EVENTS` (`serror_pending=1`, optional ESR via `KVM_CAP_ARM_INJECT_SERROR_ESR`) on `el0: serror-arm`; plus a fail-closed smoke that requires bare `el0: serror` and fails on absence. That would be a new milestone ADR.
5. **Still non-claims:** umbrella "EL0 isolated" (P-SEC-3l; [ADR-055](ADR-055-el0-isolated-checklist.md) / [ADR-060](ADR-060-isolation-leftovers-closure-checklist.md)) — out of scope here; never yank `_start` (`0x4008_0000`).

## Honesty

Say: "ADR-084 B2 spike: free GitHub-hosted runners expose no arm64 KVM/HVF/WHPX (runs 36193423604 / 36193613972, 2026-09-25); TCG there still says `machine does not provide NMIs`; B2 is Planned / blocked on a sponsor choice (self-hosted arm64 KVM, paid bare metal, or wait). Taken SError stays Verified only via the B1 opt-in pin." Do **not** say:

- B2 is Verified, or taken SError is Verified on a hosted arm64 runner
- KVM was used anywhere on ctos CI
- the B1 pin job is a B2 result
- FEAT_NMI / FIQ / BRK / a synchronous external abort is an SError
- "EL0 isolated" / "secure OS"

## Consequences

- Docs: this ADR; [ADR-081](ADR-081-taken-serror-reopen.md) follow-on; [ADR-053](ADR-053-taken-serror-hard-stop.md) / [ADR-055](ADR-055-el0-isolated-checklist.md) / [ADR-060](ADR-060-isolation-leftovers-closure-checklist.md) cross-links; honesty ledger; roadmap P-SEC-3r; threat model **v1.59**; SUMMARY; research brief + raw log excerpt + probe workflow copy under `research/daily-briefs/`; session memory under `research/random-thoughts/`.
- CI: `.github/workflows/` **unchanged** on `main` by this PR. The temporary probe branch/workflow lived only on `probe/adr-084-b2-kvm`. Existing smoke (`el0: serror-park` required) and opt-in `qemu-nmi-pin` unchanged.
- Cost: all probe minutes on free standard runners of a public repo. No paid, larger, or self-hosted runner.
