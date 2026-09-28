# ADR-085 — B2-P: taken lower-EL SError under KVM on real arm64 (AWS Graviton3 `c7g.metal`) — Verified (opt-in, one-off paid run)

- Status: Accepted. Sponsor **GO for B2-P** (paid Graviton bare metal), DSO relay 2026-09-26 11:28 CEST. Follow-on to [ADR-084](ADR-084-b2-free-runner-serror.md) §4 (B2-P). **Result: taken lower-EL SError Verified under KVM on real Graviton3 hardware** in a one-off, opt-in `b2-serror` profile (run 4, 2026-09-26 13:26–13:33 CEST). Not a CI gate: no ctos CI job runs KVM. Stock-QEMU TCG smoke still parks; the B1 opt-in pin ([ADR-083](ADR-083-b1-qemu-nmi-pin.md)) is unchanged and is **not** B2.
- Date: 2026-09-26
- Tracks P-SEC-3r (taken lower-EL SError). Builds on [ADR-081](ADR-081-taken-serror-reopen.md) §4 B2, ADR-084 K1, [ADR-045](ADR-045-taken-serror-qmp.md) standing-EL0 A-clear window, [ADR-053](ADR-053-taken-serror-hard-stop.md). CloudAgent **not used**. No self-hosted runner. Do not self-merge ([ADR-002](ADR-002-pr-identity-split.md)). Item 2 (umbrella "EL0 isolated") **not started**. It stays gated.
- **Follow-on (2026-09-26):** run 4 is the B2-P leg of the umbrella's taken-SError evidence class ([ADR-090](ADR-090-serror-evidence-class.md), sponsor D1). It is historical (at `159b178`, before the M1/M2 boot-path changes). A KVM re-verify is ≈ $0.2 and is a sponsor option, not required.

## Context

ADR-084 showed that no free GitHub-hosted runner exposes an arm64 hypervisor. The only honest B2 SError source is **K1**: a KVM host sets `serror_pending` through `KVM_SET_VCPU_EVENTS`, KVM raises a **virtual SError** (`HCR_EL2.VSE`), and the guest takes it when `PSTATE.A` is clear. That path needs real arm64 hardware booted at EL2. The sponsor chose B2-P under these constraints: a paid AWS Graviton `.metal` box, cheapest type/region, Spot, tags `project=ctos` / `component=b2-spike` on every resource, a self-terminate timer, no standing resources, stop if the estimate exceeds ~$20, and a sanctioned credentials path only.

## Design (what is different from B1)

| | B1 ([ADR-083](ADR-083-b1-qemu-nmi-pin.md)) | **B2-P (this ADR)** |
| --- | --- | --- |
| Where the guest runs | QEMU **TCG** (emulated CPU) | **Real Neoverse-V1 (Graviton3)**, QEMU `-accel kvm -cpu host` |
| Who delivers the SError | QEMU TCG raises an emulated async SError (patch 0001, `TYPE_NMI` on `virt`) | **KVM / hardware**: patch **0002** turns `inject-nmi` under `kvm_enabled()` into `env.serror.pending=1`; QEMU's resume path (`kvm_arch_put_registers` → `kvm_put_vcpu_events`) issues `KVM_SET_VCPU_EVENTS{serror_pending=1}`; KVM sets `HCR_EL2.VSE`; the CPU takes a virtual SError when EL0 runs with A clear |
| Host | GitHub `ubuntu-24.04-arm` (free) or agent box | AWS `c7g.metal`, eu-north-1b, Spot |
| Gate | opt-in CI job `qemu-nmi-pin` | one-off paid run; **no CI job** |

Pieces (branch `isolation/adr-085-b2p-graviton-kvm-serror`):

- `research/qemu-nmi/0002-hw-arm-virt-TYPE_NMI-KVM-serror-pending.patch`, applied on top of B1's 0001 on QEMU v10.0.0 via `scripts/build-qemu-b2.sh` (output `tools/qemu-b2/`, gitignored).
- Opt-in Cargo feature **`b2-serror`**: after the normal paging/identity-teardown boot, it prints `b2: boot`, runs only the standing-EL0 SError window (`el0::observe_b2_serror`), then `b2: done`. It skips GIC/timer/virtio/FAT, because the host KVM exposes GICv3 only (`kvm [1]: disabling GICv2 emulation`). It also adds raw early markers `b2: e0/e1/e2/p0/p1` and a longer outer spin (`MOVZ X3,#0xffff`) for native speed. The default build is unchanged apart from the TG1 fix below.
- `scripts/b2-kvm-smoke.py`: fail-closed. Exit 0 **only** if a bare `el0: serror` line **and** `b2: taken` appear with no park marker. If the guest is silent for 15 s it dumps vCPU registers (QMP `human-monitor-command info registers`) as diagnostics. That dump is never a pass signal.
- `scripts/b2-metal-userdata.sh`: no-SSH user-data (hard `shutdown -h +80`; `InstanceInitiatedShutdownBehavior=terminate`). It clones the branch, builds patched QEMU + the `b2-serror` guest natively, runs the smoke up to 3×, prints `CTOSB2:` lines to the serial console (read with `ec2 get-console-output --latest`), then powers off.

## Runs (2026-09-26, eu-north-1, all tagged `project=ctos`, `component=b2-spike`)

Credentials path: the sanctioned `user-Aws-mcp` `aws___run_script` (IAM user `cts`, account 737290977112). Security group `ctos-b2-spike` had **no ingress**. No key pair and no IAM role were created.

| # | Instance | Type / market | Launch (CEST) | End | Result |
| --- | --- | --- | --- | --- | --- |
| 0 | `i-09177cb42dac41e0b` | m6g.metal (Graviton2) Spot | 12:22:26 | self-poweroff ≈12:28:50 | Launched by a batch `RunInstances` whose MCP response timed out; not noticed until flagged. `HOME` unset in cloud-init → `cargo: command not found`. Patched QEMU **built with `kvm`** on Graviton2; smoke failed closed (no ELF). |
| 1 | `i-0bd798b890b40fdc4` | c7g.metal Spot | 12:23:50 | terminated by me 12:29:29 | Same `HOME` bug; terminated early (at most one metal box at a time). |
| 2 | `i-0f66c64d769c630dc` | c7g.metal Spot | 12:42:59 | self-poweroff ≈12:55:15 | Guest built. Under KVM: **zero guest UART bytes** in 3×90 s → FAIL (fail-closed). |
| 3 | `i-09ae5b56eb1f5c23b` | c7g.metal Spot | 13:06:27 | self-poweroff ≈13:17:31 | Early markers reached `b2: p1 mmu-on`, then stalled. Register dump: `PC=ffffff8040080a00` (high-alias VBAR `0xffffff8040080800` + 0x200), `PSTATE=a00003c5` EL1h, identity LR → recursive abort on the vector fetch through TTBR1. A control keeping the pre-MMU spin locks behaved identically, so the "pre-MMU exclusives" hypothesis was **refuted**. |
| 4 | `i-058d643c72be165d4` | c7g.metal Spot | 13:26:02 | self-poweroff ≈13:33:33 | **PASS** — taken lower-EL SError under KVM (below). |

Early tries at 12:1x CEST got `InsufficientInstanceCapacity` for c6g.metal Spot in eu-north-1a/b/c. Spot placement scores (read-only) then pointed to c7g.metal. ap-south-1 was cheaper but its vCPU quota is 5, so unusable.

### Root cause found on real hardware (run 3) → fix

`TCR_EL1` was programmed **without TG1**, so TG1 stayed `0b00`. That encoding is **reserved** for TG1 (01=16K, 10=4K, 11=64K; TG0 differs). QEMU TCG sanitises a reserved TG1 to 4K, so the TTBR1 high alias ([ADR-016](ADR-016-ttbr1-private-page.md)…[ADR-019](ADR-019-identity-text-range-tear.md)) always worked there. On Graviton3 the reserved value selected a different granule, so every high-VA walk failed. Fix: `TCR_TG1_4K = 0b10 << 30` in `src/paging.rs` (commit `159b178`). It applies globally because it is a latent real-hardware bug, and it is identical on TCG: default smoke at `159b178` was green on both runners ([run 36238549525](https://github.com/artofdream/ctos/actions/runs/36238549525)).

### Run 4 evidence (serial console, verbatim)

```
CTOSB2: cpu Vendor ID: ARM;BIOS Vendor ID: Amazon EC2;Model name: Neoverse-V1;BIOS Model name: AWS Graviton3 N/A CPU @ 2.6GHz;
CTOSB2: dmesg [    0.026567] CPU: All CPU(s) started at EL2
CTOSB2: dmesg [    0.228100] kvm [1]: VHE mode initialized successfully
CTOSB2: /dev/kvm=present
CTOSB2: head 159b17818fb6d22497de0544c0693844876571cc
CTOSB2: qemu build-qemu-b2: accelerators: Accelerators supported in QEMU binary: kvm tcg
CTOSB2: b2-kvm-smoke: cmd: /root/ctos/tools/qemu-b2/bin/qemu-system-aarch64 -accel kvm -machine virt,gic-version=3 -cpu host -m 128M ... -kernel target-b2/aarch64-ctos/debug/ctos
CTOSB2: b2: p1 mmu-on
CTOSB2: ident: jump
CTOSB2: pan: enabled
CTOSB2: pan: el1-fault
CTOSB2: ident: ram lo=0x4026a000 hi=0x48000000 pages=32150
CTOSB2: b2: boot (ADR-085 KVM SError profile)
CTOSB2: el0: serror-armb2-kvm-smoke: qmp stop=>ok inject-nmi=>ok cont=>ok (ADR-085: KVM_SET_VCPU_EVENTS serror_pending)
CTOSB2: el0: standing
CTOSB2: el0: serror
CTOSB2: el0: restored
CTOSB2: b2: taken
CTOSB2: b2-kvm-smoke: summary armed=True injected=True taken_marker=True b2_taken=True park=False kvm_err=False
CTOSB2: b2-kvm-smoke: PASS taken lower-EL SError observed under accel=kvm ('el0: serror' + 'b2: taken')
CTOSB2: smoke attempt 1 rc=0
```

(`el0: serror-arm` and the host's QMP log line share one console line because guest serial and host log are tee'd to the same console.) Full excerpts: [research/daily-briefs/2026-09-26-adr-085-b2p-console.txt](../../research/daily-briefs/2026-09-26-adr-085-b2p-console.txt).

The bare `el0: serror` line is printed only by the **lower-EL SError vector handler** when the standing-EL0 expectation is armed (`src/exception.rs`). Under KVM the guest runs natively, so the only SError delivery path is KVM's virtual SError that the host requested via `serror_pending`. Limitation: no host-side `strace` of the ioctl was captured. The mechanism is QEMU's documented `kvm_put_vcpu_events` path, and the fail-closed marker was observed.

## Cost

Spot, per-second billing. Total metal time across runs 0–4 ≈ 43 min: c7g.metal ≈ 36.6 min at $0.2476/h plus m6g.metal ≈ 6.4 min at ≈$0.26/h. Prices come from `DescribeSpotPriceHistory` in eu-north-1b, 2026-09-26. Metal ≈ **$0.18**. Add gp3 30 GB and public IPv4 for ≈0.7 h (< $0.01). **Total ≈ $0.19**, far under the $20 gate.

## Decision

1. **B2 (different accelerator: real arm64 + KVM): Verified, opt-in, one-off.** It rests on run 4 above, a probe from this session. It is **not** a standing CI gate. Re-running needs another paid metal run (user-data script in-tree) or a self-hosted arm64 KVM runner (B2-S, still not registered).
2. **Stock QEMU TCG smoke:** unchanged. It still requires `el0: serror-park` (ADR-053/081).
3. **B1 pin:** unchanged, Verified (opt-in, TCG). Still **not** B2.
4. **TG1=4K** is a real correctness fix for real hardware. The TTBR1 alias was only "Verified" on TCG before this ADR.
5. **Still non-claims:** umbrella "EL0 isolated" (P-SEC-3l; ADR-055/060) is not claimed and not started. `_start` stays at `0x4008_0000`. This is not "ctos runs on real hardware" in general: only the `b2-serror` profile ran, with no GIC/timer/virtio/FAT.

## Honesty

Say: "ADR-085 B2-P: on an AWS Graviton3 `c7g.metal` (eu-north-1, Spot, 2026-09-26), QEMU `-accel kvm` with patch 0002 set `serror_pending` via `KVM_SET_VCPU_EVENTS` on the `el0: serror-arm` cue, and the ctos `b2-serror` profile took the lower-EL SError while standing at EL0 (`el0: serror` / `b2: taken`, fail-closed smoke PASS). One-off paid run, not CI. Found and fixed a latent TCR_EL1.TG1 bug that TCG masked." Do **not** say:

- taken SError is Verified on stock QEMU, on free runners, or in CI under KVM
- the default ctos image boots on real hardware (only the b2 profile ran; GIC/timer/virtio untested under KVM)
- the B1 pin job is B2 evidence
- "EL0 isolated" / "secure OS"

## Consequences

- Code: opt-in `b2-serror` feature (`src/main.rs`, `src/el0.rs`, early markers in `src/paging.rs`); global `TCR_TG1_4K`; patch 0002; `scripts/build-qemu-b2.sh`, `scripts/b2-kvm-smoke.py`, `scripts/b2-metal-userdata.sh`; `.gitignore` `/tools/qemu-b2/`.
- CI: `.github/workflows/` unchanged. Default smoke and opt-in `qemu-nmi-pin` stay green (run 36238549525).
- Docs: this ADR; ADR-084 follow-on; honesty ledger; roadmap P-SEC-3r; threat model **v1.60**; SUMMARY; research brief + console excerpt; session memory.
- AWS: all resources deleted; tag audit in the research brief.
