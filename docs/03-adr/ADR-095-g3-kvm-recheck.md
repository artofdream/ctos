# ADR-095 — G3: B2-P KVM taken-SError re-run on current `main` (AWS Graviton3 `c7g.metal`) — PASS

- Status: Accepted (evidence record, docs only). The sponsor asked for **G3**, the optional B2-P KVM re-run listed as the only remaining limit in [ADR-091](ADR-091-el0-isolated-accept-draft.md) revision 3 (DSO relay 2026-09-28). **Result: PASS on the first smoke attempt.** The taken lower-EL SError still happens under KVM on real Graviton3 hardware on current `main`, the same way [ADR-085](ADR-085-b2p-graviton-kvm-serror.md) run 4 showed it at `159b178`. ADR-091 stays **Draft**; B3 (the sponsor's written accept) is **not Met**; the umbrella “EL0 isolated” stays **Planned / non-claim**. Nothing is flipped here.
- Date: 2026-09-28 (launch 13:50:32 CEST, smoke PASS before 13:58:13 CEST, teardown verified ≈ 14:06 CEST).
- Tested commit: `main` at **`8107578a6c715a89c9e5861ebb4102460ba20e80`** (merge of PR #146, ADR-094). It contains the whole EL0-isolation stack: ADR-087 … ADR-092 and ADR-094, including the ADR-090 evidence class and the ADR-091 draft.
- **`main` moved during review (2026-09-28):** PR #145 (ADR-093) merged as `e1ee4a8` after this run, and this branch merged it. Between the tested `8107578` and `e1ee4a8` there is **no** change to `src/`, `user/`, `libctos/`, `Cargo.*`, `build.rs`, `linker.ld`, the target JSON, `rust-toolchain.toml`, `research/qemu-nmi/` or the three B2 scripts (empty `git diff`), so the guest and QEMU tested here are byte-identical in source to the current tip. ADR-093 changed only docs, `scripts/check-public-ids.sh` and a guard hook in `scripts/qemu-smoke.sh`.
- No `src/`, `scripts/` or `.github/workflows/` change. The in-tree `scripts/b2-metal-userdata.sh` is unchanged; the run used a copy with the small delta listed below. `_start` (`0x4008_0000`) untouched. CloudAgent not used. Do not self-merge ([ADR-002](ADR-002-pr-identity-split.md)).
- Public-identifier rule ([ADR-093](ADR-093-public-identifier-scrub-guard.md), PR #145): no account, instance, Spot request, security group, volume, network, image or user identifiers appear in this record. They are described in words.

## Context

ADR-085 run 4 (2026-09-26) is the B2-P leg of the ADR-090 taken-SError evidence class. It ran at `159b178`, before the M1/M2 boot-path changes (ADR-087 RAM-side identity tears, ADR-088 TTBR1 MMIO high alias + identity MMIO tear) and before ADR-092 / ADR-094. ADR-090 therefore carries the caveat “B2-P is historical”, and ADR-091 revision 3 lists **G3: B2-P re-verify on the post-M2 boot path (≈ $0.2 paid run)** as its only remaining limit. Since then the `b2-serror` profile was only checked under stock TCG (ADR-087 / ADR-088 / ADR-090), where it parks as expected.

## What was run

Same procedure as ADR-085 run 4, adapted to current `main`:

| | ADR-085 run 4 | **ADR-095 G3 (this ADR)** |
| --- | --- | --- |
| ctos commit | `159b178` (branch head) | **`8107578`** (`main`, detached checkout of the full SHA) |
| Host | AWS `c7g.metal` (Graviton3), eu-north-1b, one-time Spot, max price $0.60 | same |
| OS image | Ubuntu 24.04 arm64 | Canonical Ubuntu 24.04 LTS arm64 server (gp3), build 20260923 |
| Root volume | gp3 30 GB, delete on termination | same |
| Access | no SSH, no key pair, no IAM role; security group with **no ingress**; serial console only | same; IMDSv2 required |
| Tags (every resource) | `project=ctos`, `component=b2-spike` | `project=ctos`, **`component=g3-kvm-recheck`** on the instance, root volume, network interface, Spot request and security group |
| Hard stop | `shutdown -h +80` + shutdown behaviour `terminate` | `shutdown -h +60` + shutdown behaviour `terminate`; the user-data powers off right after the smoke |
| QEMU | v10.0.0 `7c949c53` + patches 0001 + 0002 via `scripts/build-qemu-b2.sh` | same script, same pin |
| Guest | `cargo build --features b2-serror --target-dir target-b2` | same |
| Smoke | `scripts/b2-kvm-smoke.py` (fail-closed), `CTOS_B2_TIMEOUT=90`, `CTOS_B2_STALL=15`, up to 3 attempts | same |

Credentials path: the sanctioned AWS MCP `run_script` tool with the ctos-scoped IAM user (no other project's credentials). Each mutating call ran in its own script, and every launch re-checked that no `project=ctos` instance was pending, running, stopping, stopped or shutting down (ADR-085 lesson: never two metal boxes).

AWS call sequence (eu-north-1):

1. Read-only: `DescribeInstances` (guard: 0 active ctos instances), `DescribeSpotPriceHistory` (c7g.metal eu-north-1b $0.2476/h, unchanged from ADR-085), `DescribeImages` (latest Canonical Ubuntu 24.04 arm64), `DescribeSubnets` / `DescribeVpcs` (default VPC), `DescribeSecurityGroups` (no leftover ctos group).
2. `CreateSecurityGroup` `ctos-g3-kvm-recheck` in the default VPC, tagged at create. No ingress rule was added (default egress only).
3. `DescribeInstances` guard, then one `RunInstances`: `c7g.metal`, one-time Spot, user-data below, tags on instance / volume / network interface / Spot request. Launch **13:50:32 CEST**.
4. Read-only polling: `GetConsoleOutput` (`Latest=true`) and `DescribeInstances`.
5. After the instance self-terminated: `DeleteSecurityGroup`, then the verification describes in [Teardown](#teardown).

User-data = `scripts/b2-metal-userdata.sh` with this delta (nothing else changed):

```text
- shutdown -h +80 "ctos b2 hard timer" || true
+ shutdown -h +60 "ctos g3 hard timer" || true
- echo "CTOSB2: uname $(uname -a)"
+ echo "CTOSB2: uname $(uname -srm)"                      # no hostname on the console
+ echo "CTOSB2: kvm-api $(python3 -c "import fcntl,os;print(fcntl.ioctl(os.open('/dev/kvm',os.O_RDWR),0xAE00))" 2>&1)"   # KVM_GET_API_VERSION
- git clone --depth 1 --branch "__BRANCH__" https://github.com/artofdream/ctos.git ctos
+ git clone --branch main https://github.com/artofdream/ctos.git ctos && git -C ctos checkout --detach 8107578a6c715a89c9e5861ebb4102460ba20e80
+ echo "CTOSB2: qemu-version $(tools/qemu-b2/bin/qemu-system-aarch64 --version | head -1)"
```

Pre-flight (free, agent box, before paying): `cargo build --features b2-serror` at `8107578`, then `CTOS_B2_ACCEL=tcg python3 scripts/b2-kvm-smoke.py` with stock QEMU 10.0.13. The guest booted through `ident: mmio-high` … `ident: mmio` to `b2: boot` → `el0: serror-arm`; stock QEMU answered `inject-nmi=>error:machine does not provide NMIs`, so the guest printed `el0: serror-park` / `b2: not taken` and the smoke **failed closed**, which is the expected stock-TCG result. It showed the profile still boots on current `main`; it is not SError evidence.

## Versions (from the serial console)

- CPU: `Neoverse-V1` / `AWS Graviton3 … @ 2.6GHz`; `CPU: All CPU(s) started at EL2`; `kvm [1]: VHE mode initialized successfully`.
- Host kernel: `Linux 7.0.0-1013-aws aarch64` (Ubuntu 24.04 AWS kernel). KVM is the in-kernel module of that kernel; `KVM_GET_API_VERSION` = **12**; vGIC is GICv3 only (`disabling GICv2 emulation`), `IPA Size Limit: 48 bits`.
- QEMU: `QEMU emulator version 10.0.0 (v10.0.0-dirty)` (dirty = patches 0001 + 0002 applied), `Accelerators supported in QEMU binary: kvm tcg`. Guest command: `-accel kvm -machine virt,gic-version=3 -cpu host -m 128M …`.
- Rust: `nightly-2026-09-12` from `rust-toolchain.toml`, auto-installed by `rustup 1.29.1`. Honest note: the `CTOSB2: rustc` line captured rustup's “syncing channel updates for nightly-2026-09-12-aarch64-unknown-linux-gnu” message instead of a `rustc --version` string; the toolchain path in the cargo warning confirms the same pin. `cargo rc=0`, `qemu-build rc=0`.

## Evidence (serial console, verbatim)

```text
CTOSB2: uname Linux 7.0.0-1013-aws aarch64
CTOSB2: cpu Vendor ID: ARM;BIOS Vendor ID: Amazon EC2;Model name: Neoverse-V1;BIOS Model name: AWS Graviton3 N/A CPU @ 2.6GHz;
CTOSB2: dmesg [    0.026519] CPU: All CPU(s) started at EL2
CTOSB2: dmesg [    0.227753] kvm [1]: VHE mode initialized successfully
CTOSB2: /dev/kvm=present
CTOSB2: kvm-api 12
CTOSB2: head 8107578a6c715a89c9e5861ebb4102460ba20e80
CTOSB2: qemu-version QEMU emulator version 10.0.0 (v10.0.0-dirty)
CTOSB2: qemu build-qemu-b2: accelerators: Accelerators supported in QEMU binary: kvm tcg
CTOSB2: ===== smoke attempt 1 =====
CTOSB2: b2-kvm-smoke: cmd: /root/ctos/tools/qemu-b2/bin/qemu-system-aarch64 -accel kvm -machine virt,gic-version=3 -cpu host -m 128M -display none -serial stdio -monitor none -qmp unix:/tmp/ctos-b2-yyvqz4_3/qmp.sock,server,nowait -kernel target-b2/aarch64-ctos/debug/ctos
CTOSB2: b2: p1 mmu-on
CTOSB2: ident: jump
CTOSB2: ident: mmio-high gic=0xffffff8008000000 uart=0xffffff8009000000 virtio=0xffffff800a000000
CTOSB2: pan: id=2
CTOSB2: pan: present
CTOSB2: pan: enabled
CTOSB2: pan: el1-fault
CTOSB2: ident: ram lo=0x4026b000 hi=0x48000000 pages=32149
CTOSB2: ident: low lo=0x40000000 hi=0x40080000 pages=128 user=0
CTOSB2: ident: tail lo=0x400ca000 hi=0x40201000 pages=311 user=311
CTOSB2: ident: kend lo=0x4025a000 hi=0x4025b000 pages=1 user=0
CTOSB2: ident: mmio lo=0x0 hi=0x40000000 l1=1
CTOSB2: b2: boot (ADR-085 KVM SError profile)
CTOSB2: el0: serror-armb2-kvm-smoke: qmp stop=>ok inject-nmi=>ok cont=>ok (ADR-085: KVM_SET_VCPU_EVENTS serror_pending)
CTOSB2: 
CTOSB2: el0: standing
CTOSB2: el0: serror
CTOSB2: el0: restored
CTOSB2: b2: taken
CTOSB2: b2: doneb2-kvm-smoke: summary armed=True injected=True taken_marker=True b2_taken=True park=False kvm_err=False
CTOSB2: b2-kvm-smoke: PASS taken lower-EL SError observed under accel=kvm ('el0: serror' + 'b2: taken')
CTOSB2: smoke attempt 1 rc=0
CTOSB2: end 2026-09-28T11:58:13Z
```

(`el0: serror-arm` + host QMP line and `b2: done` + summary share console lines because guest serial and host log are tee'd to the same console, as in ADR-085.) Full `CTOSB2:` excerpt (66 lines, including the early markers `b2: e0 … p1` and every `ident:` line): [research/daily-briefs/2026-09-28-adr-095-g3-console.txt](../../research/daily-briefs/2026-09-28-adr-095-g3-console.txt).

Reading: the guest ran natively on Graviton3 under KVM through the **current** boot path: `ident: mmio-high` (ADR-088 TTBR1 Device alias), `ident: low` / `tail` / `kend` (ADR-087 RAM-side tears), `ident: mmio … l1=1` (ADR-088 identity MMIO tear), PAN enable and EL1 fault. On the `el0: serror-arm` cue QEMU (patch 0002) turned `inject-nmi` into `KVM_SET_VCPU_EVENTS{serror_pending=1}`; KVM raised a virtual SError (`HCR_EL2.VSE`), and the guest took it at the **lower-EL SError vector** while standing at EL0: the bare `el0: serror` line is printed only by that handler with the standing-EL0 expectation armed (`src/exception.rs`). No `el0: serror-park`, no `b2: not taken`, fail-closed smoke PASS on attempt 1.

Difference from TCG: Graviton3 reports `pan: id=2` (`ID_AA64MMFR1_EL1.PAN` = 2); PAN enable and the EL1-vs-EL0 fault still pass. `ident: ram … pages=32149` vs `32150` in run 4 follows the image growth since `159b178`.

## Cost

One c7g.metal Spot instance, per-second billing, $0.2476/h (`DescribeSpotPriceHistory`, eu-north-1b, flat on 2026-09-28). Running 13:50:32 → self-poweroff ≈ 13:58:35 CEST (console `end 11:58:13Z` + 20 s sleep) ≈ 8.1 min ≈ **$0.033**. The instance then sat in `shutting-down` and was `terminated` by ≈ 14:06 CEST; even billing that whole 15.5-minute window at the Spot rate gives only ≈ $0.064. gp3 30 GB and one public IPv4 for the same window: < $0.01. **Total ≈ $0.04 (upper bound ≈ $0.07)**, under the ≈ $0.2 budget and far under the $20 stop gate.

## Teardown

- The instance powered itself off after the smoke and terminated (`Client.InstanceInitiatedShutdown`; shutdown behaviour `terminate`). The root volume was delete-on-termination; the network interface went with the instance; the one-time Spot request closed.
- The security group was deleted after the instance reached `terminated`.
- Verification (≈ 14:06 CEST, read-only describes, eu-north-1): `project=ctos` instances pending / running / stopping / stopped / shutting-down → **0**; the run's instance → `terminated`; `project=ctos` volumes → **0**, and the run's root volume by ID → `InvalidVolume.NotFound`; tagged network interfaces → **0**; `project=ctos` security groups → **0**, and the run's group by ID → `InvalidGroup.NotFound`; tagged key pairs → **0**; tagged snapshots → **0**; tagged Elastic IPs → **0**; the one `component=g3-kvm-recheck` Spot request → `closed` / `instance-terminated-by-user` (a closed one-time request is not billable and ages out by itself).
- No key pair, IAM role, instance profile, snapshot, AMI or Elastic IP was created.

## Decision

1. **G3: done, PASS.** The B2-P leg of the ADR-090 evidence class is re-verified on current `main` (`8107578`) under KVM on real Graviton3 hardware, same host class, same QEMU pin and patches, same fail-closed smoke. ADR-091 gap G3 is closed with a pointer here.
2. **ADR-090 §1 is unchanged.** It is the sponsor's verbatim D1 paragraph and still names ADR-085 run 4. This ADR is a supplementary re-run of that leg on the post-M2 boot path. The ADR-090 caveat “B2-P is historical (at `159b178`)” is answered by this record; the caveats “reduced profile” and “not real hardware running the full default image” still stand.
3. **ADR-091 sentence: no change needed.** The revision-3 sentence says taken lower-EL SError “is evidenced by the ADR-090 evidence class” and that the sentence is “not a real-hardware claim”. Both stay true and complete. G3 was a limit on the evidence, not on the wording.
4. **Not changed:** ADR-091 stays Draft; B3 not Met; umbrella Planned / non-claim; no CI job runs KVM; the default smoke on stock QEMU still requires `el0: serror-park`; the B1 pin is unchanged.

## What this does not show

- **Reduced profile only.** `b2-serror` skips GIC / timer / virtio / FAT, and it does not run the identity inventory (`ident: inv`), the EL0-reachability walk (ADR-092), the EL0 write-fault probes or the syscall-pointer probes (ADR-094). Those are CI-gated on TCG only.
- **One-off, not CI.** Re-running needs another paid metal run or a self-hosted arm64 KVM runner (B2-S, not registered).
- No host-side `strace` of the ioctl (same limitation as ADR-085); the mechanism is QEMU's `kvm_put_vcpu_events` path and the fail-closed marker was observed.
- Not “the default ctos image runs on real hardware”, not “EL0 isolated”, not “secure OS”.

## Honesty

Say: “ADR-095 (G3): on 2026-09-28 an AWS Graviton3 `c7g.metal` (eu-north-1b, Spot) re-ran the ADR-085 B2-P procedure on ctos `main` `8107578`; QEMU `-accel kvm` with patch 0002 set `serror_pending` via `KVM_SET_VCPU_EVENTS` on the `el0: serror-arm` cue, and the `b2-serror` profile took the lower-EL SError while standing at EL0 (`el0: serror` / `b2: taken`, fail-closed smoke PASS, attempt 1). One-off paid run, not CI, reduced profile. ADR-091 G3 is closed; ADR-091 is still a draft.” Do **not** say:

- taken SError is Verified in CI under KVM, on stock QEMU, or on free runners
- the default image boots on real hardware
- “EL0 isolated”, “B3 met”, or “ADR-091 accepted”

## Consequences

- Docs: this ADR; ADR-091 gap G3 marked done (pointer here), sentence untouched, status untouched; honesty-ledger rows; threat model **v1.69** note; SUMMARY; research MOC; console excerpt [research/daily-briefs/2026-09-28-adr-095-g3-console.txt](../../research/daily-briefs/2026-09-28-adr-095-g3-console.txt).
- Code / scripts / CI: unchanged. CI on PR #147: honesty-ledger row “CI on GitHub (ADR-095 / PR #147)”.
- AWS: everything created for the run is deleted and verified (above).
