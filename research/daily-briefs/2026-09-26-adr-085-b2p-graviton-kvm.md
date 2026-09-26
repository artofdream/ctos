# 2026-09-26 — ADR-085 B2-P: taken SError under KVM on AWS Graviton3 (Verified, opt-in, one-off)

Handoff brief. Sponsor GO for B2-P relayed by DSO 2026-09-26 11:28 CEST. Decision record: [ADR-085](../../docs/03-adr/ADR-085-b2p-graviton-kvm-serror.md). Raw console: [2026-09-26-adr-085-b2p-console.txt](2026-09-26-adr-085-b2p-console.txt).

## Outcome

- **Taken lower-EL SError observed under KVM on real arm64: YES.** Run 4, `c7g.metal` `i-058d643c72be165d4`, eu-north-1b, Spot, 13:26:02–≈13:33:33 CEST, head `159b178`. Bare `el0: serror` + `b2: taken` + `b2-kvm-smoke: PASS ... accel=kvm`.
- SError source: `KVM_SET_VCPU_EVENTS{serror_pending=1}` requested by QEMU (patch 0002) on the guest's `el0: serror-arm` cue. KVM delivers it as a virtual SError (`HCR_EL2.VSE`). This is not TCG and not the B1 pin.
- Real-hardware bug found and fixed: `TCR_EL1.TG1` was the reserved `0b00`. TCG sanitised it to 4K; Graviton3 did not, so every TTBR1 high-VA walk failed. Now `TG1=4K` (`src/paging.rs`).

## Price and choice (checked before launch)

`DescribeSpotPriceHistory` 2026-09-26 (Linux/UNIX, per hour): c6gd.metal ap-south-1 $0.1827 and c6g.metal ap-south-1 $0.2311 (vCPU quota 5, unusable), c6g.metal eu-north-1 $0.2336, c7g.metal eu-north-1b **$0.2476**, m6g.metal eu-north-1 $0.2624, a1.metal us-east-2 $0.355. c6g.metal Spot returned `InsufficientInstanceCapacity` in eu-north-1a/b/c. `GetSpotPlacementScores` (read-only) then led to c7g.metal in eu-north-1b. The pre-launch estimate (1–2 h Spot) was < $1, far under the $20 gate.

## Instances (all tagged `project=ctos`, `component=b2-spike`; volumes + ENI + Spot request tagged at create)

Hard timer on every instance: `shutdown -h +80` in user-data, plus `InstanceInitiatedShutdownBehavior=terminate`. One-time Spot, MaxPrice $0.60, gp3 30 GB `DeleteOnTermination`, IMDSv2 required, SG `ctos-b2-spike` (`sg-0ada33a802e97e6ce`, no ingress). No key pair, no IAM role.

| Run | Instance | Type | Launch (CEST) | Shutdown / termination | Timer deadline (CEST) | Minutes |
| --- | --- | --- | --- | --- | --- | --- |
| 0 | i-09177cb42dac41e0b | m6g.metal | 12:22:26 | self-poweroff ≈12:28:50 (end 12:28:29 + 20 s) | 13:42 | ≈6.4 |
| 1 | i-0bd798b890b40fdc4 | c7g.metal | 12:23:50 | TerminateInstances 12:29:29 | 13:44 | ≈5.7 |
| 2 | i-0f66c64d769c630dc | c7g.metal | 12:42:59 | self-poweroff ≈12:55:15 (end 12:54:53) | 14:03 | ≈12.3 |
| 3 | i-09ae5b56eb1f5c23b | c7g.metal | 13:06:27 | self-poweroff ≈13:17:31 (end 13:17:11) | 14:26 | ≈11.1 |
| 4 | i-058d643c72be165d4 | c7g.metal | 13:26:02 | self-poweroff ≈13:33:33 (end 13:33:13) | 14:46 | ≈7.5 |

Run 0 came from a batch `RunInstances` whose MCP response timed out. A follow-up describe did not show it yet (eventual consistency), so runs 0 and 1 overlapped for ≈5 min until the sponsor flagged it. After that, a launch guard refused to launch while any `project=ctos` instance was pending/running/shutting-down, so at most one metal box was alive at a time.

Cost: c7g.metal ≈36.6 min × $0.2476/h ≈ $0.151; m6g.metal ≈6.4 min × $0.2624/h ≈ $0.028; gp3 + public IPv4 < $0.01. **≈ $0.19 total.** Source: `DescribeSpotPriceHistory` eu-north-1b; prices were flat over 09:00–11:45 UTC.

## Bring-up log (what each run taught)

1. Runs 0–1: cloud-init runs user-data with `HOME` unset, so `. "$HOME/.cargo/env"` failed. Fix: `export HOME=/root` + PATH (`7accb97`).
2. Run 2: the guest printed nothing under KVM. TCG with cortex-a76/max/neoverse-v1/n1 on the agent box printed normally, so this was a real-hardware difference.
3. Run 3: raw early markers (`b2: e0 … p1`) and a stall register dump. Hang after MMU-on at high VBAR+0x200. A control with the original pre-MMU spin locks behaved identically, so the exclusives hypothesis was refuted and reverted. Root cause: TCR TG1 reserved.
4. Run 4: TG1=4K → full b2 boot under KVM → taken SError PASS on attempt 1.

## CI

Stacked branch `isolation/adr-085-b2p-graviton-kvm-serror` (on top of PR #136's branch). GHA smoke at `159b178`: [36238549525](https://github.com/artofdream/ctos/actions/runs/36238549525) success. Default jobs still `inject-nmi=>error:machine does not provide NMIs` → `el0: serror-park` → `qemu-smoke: ok`; the opt-in `qemu-nmi-pin` job shows bare `el0: serror` (B1). No workflow change.

## Cleanup (audit 2026-09-26 ≈13:38 CEST, read-only calls via `user-Aws-mcp`)

- SG `sg-0ada33a802e97e6ce` (`ctos-b2-spike`): `DeleteSecurityGroup` succeeded after the last instance terminated. It was earlier also created and deleted once around the first capacity failure. No key pair, IAM role or instance profile was created. The only IAM substring hit, `serverlessrepo-3dx-lab-…bb2SKAy…`, is unrelated and pre-existing.
- eu-north-1, us-east-1, us-east-2, ap-south-1: `DescribeInstances` with `tag:project=ctos` in pending/running/stopping/stopped/shutting-down → **0**. Tagged volumes → **0**. Tagged ENIs → **0**. SG `ctos-b2-spike` → **0**. Tagged key pairs → **0**. Open/active tagged Spot requests → **0**.
- `resourcegroupstaggingapi GetResources Key=project,Values=ctos`: us-east-1 / us-east-2 / ap-south-1 → **0**. eu-north-1 still indexes 20 ARNs: 5 instances, 5 volumes, 5 ENIs, 5 Spot requests. Checked by ID: all 5 volumes `InvalidVolume.NotFound`, all 5 ENIs `InvalidNetworkInterfaceID.NotFound`, instances `terminated` (i-0bd798b890b40fdc4 already aged out), all 5 Spot requests `closed / instance-terminated-by-user`. These are stale tagging-index entries for deleted or terminated resources. Nothing is running or billable.
