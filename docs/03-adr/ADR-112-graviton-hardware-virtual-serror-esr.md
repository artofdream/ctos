# ADR-112 — Real-hardware SError milestone: what Graviton metal can honestly give, and the plan (KVM **virtual SError** with a host-chosen syndrome)

- Status: **Proposed (docs only).** Nothing here is implemented. **No AWS call that creates, changes or launches anything was made** to write it; no instance was launched. The sponsor decides; do not self-merge ([ADR-002](ADR-002-pr-identity-split.md)).
- Date: 2026-11-12 (CET). Request: the user, 2026-11-12 22:15 CET, "proceed with next milestone including real-hardware SError".
- Base: `main` `9ebef21` (merge of #170). ADR-107 … ADR-111 are taken, so this is ADR-112.
- Follows: [ADR-085](ADR-085-b2p-graviton-kvm-serror.md) (B2-P, first KVM run), [ADR-090](ADR-090-serror-evidence-class.md) (evidence class), [ADR-095](ADR-095-g3-kvm-recheck.md) (G3 re-run), [ADR-099](ADR-099-kvm-recheck-ci-oidc-sketch.md) (OIDC workflow, pilot), [ADR-108](ADR-108-taken-serror-qemu-spike.md) (QEMU spike), [ADR-111](ADR-111-n2-el2-shim-virtual-serror.md) (EL2 shim **virtual SError**).
- Not in this ADR: no `src/`, `user/`, `libctos/`, `Cargo.*`, `scripts/` or `.github/workflows/` change. `_start` (`0x4008_0000`) untouched. No claim sentence changes (ADR-091 rev 4 and ADR-106 stay as accepted). Public-identifier rule ([ADR-093](ADR-093-public-identifier-scrub-guard.md)): no account, role, instance, image, subnet or security-group value appears here.

## Context

"Real-hardware SError" has been asked for since ADR-081. What exists today:

| Evidence | What took the SError | Who raised it |
| --- | --- | --- |
| B1 pin (ADR-083/090), N1 (ADR-110) | ctos at EL0, QEMU TCG | QEMU code this repo patches (emulated) |
| N2 shim (ADR-111) | ctos at EL0, stock QEMU TCG | emulated `HCR_EL2.VSE` (**virtual SError**) |
| B2-P / G3 / ADR-099 pilot (ADR-085/095/099) | ctos `b2-serror` profile at EL0, **real Graviton3 silicon** under KVM | host QEMU → `KVM_SET_VCPU_EVENTS{serror_pending=1}` → KVM sets `HCR_EL2.VSE` → the CPU delivers a **virtual SError** |

So a hardware-delivered **virtual SError** on Graviton3 is already shown three times. Every one of those runs let KVM choose the syndrome, and none of them checked it. The open question for this milestone is whether ctos can go further on Graviton metal: a **physical** SError, a host-chosen syndrome, or ctos on bare metal with no hypervisor.

## Research (2026-11-12): what each route can and cannot give

Sources are listed at the end ([S1]–[S10]). "Graviton3" = AWS `c7g.metal` / `m7g.metal`, Arm Neoverse V1 (ADR-085 console: `Model name: Neoverse-V1`). Neoverse V1 implements FEAT_RAS.

### R1. ctos as a KVM guest, host-injected **virtual SError** with a chosen ESR (`serror_has_esr`)

- `KVM_SET_VCPU_EVENTS` on arm64 takes `serror_pending`, `serror_has_esr` and `serror_esr`. "Some CPUs have the ability to specify what the virtual SError's ESR value should be. These systems will advertise `KVM_CAP_ARM_INJECT_SERROR_ESR`." Only the low 24 ISS bits may be set; anything else is `-EINVAL` [S1]. In the kernel, `has_esr` without `ARM64_HAS_RAS_EXTN` is `-EINVAL`; with it, `kvm_inject_serror_esr()` loads the value into `VSESR_EL2` [S2]. With `has_esr=0`, KVM uses ISS = IDS only (bit 24, "no valid syndrome") [S2].
- QEMU v10.0.0 already passes `env->serror.has_esr` / `esr` to KVM, but **only if** it found `KVM_CAP_ARM_INJECT_SERROR_ESR`; otherwise it drops them silently [S3]. Patch 0002 (ADR-085) always sets `has_esr = 0`.
- **Can claim:** real Graviton3 hardware delivered a **virtual SError** to ctos at EL0, carrying a syndrome the host chose for this run, and ctos read that exact syndrome from `ESR_EL1` (EC `0x2f`). That is new: earlier runs never checked the syndrome.
- **Cannot claim:** a physical SError, a RAS error, a hardware fault, or that ctos handles real errors. KVM writes `VSESR_EL2`; the CPU only delivers it.

### R2. Can a ctos guest trigger a **physical** SError?

- **Unmapped PA / IPA.** A guest never sees a PA; stage-2 sits in between. An access to an IPA with no memslot exits to QEMU as `KVM_EXIT_MMIO`. QEMU v10.0.0 calls `address_space_rw()` and **ignores the result** [S4], so an unassigned access reads as zero / writes are ignored. No abort and no SError reach the guest. (ADR-108 already saw that a bad device access under TCG is a *synchronous* abort, not an SError.)
- **RAS error-injection registers (`ERXPFGCTL_EL1` …).** On Neoverse V1 the core can inject CE, DE or UC pseudo-faults through `ERR0PFGCTL` [S5][S6]. But: (a) the registers are reachable at EL1/EL2 only if **EL3 firmware** sets `ACTLR_EL3[5]` (and `SCR_EL3.TERR` is clear) [S5]; on AWS that firmware is not ours and not documented. (b) KVM sets `HCR_EL2.TERR` on RAS CPUs, so a guest's `ERX*` access traps, and KVM emulates every error-record register as **RAZ/WI** (`access_ras` → `trap_raz_wi`) [S7]. A ctos guest therefore sees **no error records** and cannot inject.
- **Host-side injection (a kernel module on the metal Linux).** Even if firmware allowed it, a physical SError taken while the guest runs goes to EL2 (`HCR_EL2.AMO`). KVM's `kvm_handle_guest_serror()` [S8] and `arm64_is_fatal_ras_serror()` [S9] decide: **UC → host panic** (`arm64_serror_panic`); **UEU/UER → KVM injects a virtual SError** into the guest; **CE/UEO → nothing for the guest**. Neoverse V1 injects only CE, DE and UC [S6]. So the best case is a host panic, and in every case ctos would still see at most a **virtual SError**. It would also put deliberate faults into the RAS logs of rented AWS hardware, with unknown effects on AWS host-health handling. **Not proposed.**
- **ACPI EINJ / firmware-first.** EINJ is a host firmware table, if Graviton has one (unknown; a read-only probe is proposed below). A memory error that a guest consumes comes back as a *synchronous* external abort through APEI/KVM (`kvm_handle_guest_sea`) [S10], not as an SError. **Not proposed.**
- **Can claim from R2:** only the negative, as a recorded finding: on Graviton metal under KVM, ctos cannot raise or receive a physical SError; KVM turns any non-fatal guest-time physical SError into a **virtual SError** or panics the host.

### R3. ctos bare-metal on `.metal` (no hypervisor)

- Graviton instances boot UEFI only and need an OS with ACPI tables and ACPI PCI hot-plug [S11][S12]. ctos is a raw ELF for QEMU `virt`: no PE/COFF UEFI loader, no ACPI parsing, a fixed load address (`_start` at `0x4008_0000`, which must not change), a `virt` PL011 address, GICv2 in the default image (ADR-085: the host offers GICv3 only), and no NVMe/ENA driver. The only output would be `GetConsoleOutput` after the fact.
- Even fully ported, bare-metal ctos would have no controllable physical SError source beyond R2's pseudo-fault path, which depends on unknown EL3 firmware settings.
- **Verdict:** a separate multi-milestone port, not this milestone. **Cannot claim** anything about bare-metal Graviton from this ADR.

## Options, ranked

| Rank | Option | New evidence | Cost / risk | Verdict |
| --- | --- | --- | --- | --- |
| 1 | **K-ESR** (R1): KVM **virtual SError** with host-chosen ESR, checked in the guest; plus read-only host RAS inventory | Syndrome round-trip on silicon; RAS facts on record | One Spot run, ≈ $0.06 expected, $0.33 cap; small patch + feature | **Chosen** |
| 2 | Plain re-run of ADR-085/099 at current `main` | Freshness only | Same | Folded into 1 (row KD) |
| 3 | Host pseudo-fault module (R2) | At best a host panic; guest still sees **virtual SError** | Firmware-dependent; AWS host-health risk | Rejected |
| 4 | EINJ (R2) | Synchronous abort, not SError | Firmware-dependent | Rejected (inventory only) |
| 5 | Bare-metal ctos (R3) | Boot port, not SError | Months; `_start` conflict | Out of scope |

## Decision (proposed)

Milestone **HW-VSE**: on one Graviton3 `.metal` Spot instance, under KVM, ctos takes a **virtual SError** at EL0 whose syndrome was chosen by the host for this run, and the run records what the hardware and KVM expose for RAS.

### Pieces (to implement in a later PR, after this ADR is accepted)

1. **QEMU patch `research/qemu-nmi/0003-…-serror-esr.patch`** (on top of 0001+0002, QEMU v10.0.0). Adds a `virt` machine property `x-ctos-serror-esr` (hex, default unset).
   - Unset: exactly patch 0002 (`has_esr = 0`).
   - Set: `virt_nmi` under KVM sets `serror.has_esr = 1`, `serror.esr = <value>`.
   - If KVM lacks `KVM_CAP_ARM_INJECT_SERROR_ESR`, `inject-nmi` returns an error (no silent fallback; QEMU would otherwise drop the ESR [S3]).
2. **Cargo feature `b2-serror-esr`** (opt-in, implies `b2-serror`).
   - In the lower-EL SError handler, print `el0: virtual serror taken (kvm esr) ec=0x2f iss=0x<ESR_EL1[24:0]>` instead of the bare `el0: serror`.
   - Before the window, print `ras: pfr0=<ID_AA64PFR0_EL1.RAS>`; only if that field is non-zero, read `ERRIDR_EL1` and print `ras: erridr=<value>` (KVM is expected to give 0, RAZ). With RAS 0 it prints `ras: erridr=skip` and never touches the register (it would UNDEF).
   - Default build and `b2-serror` unchanged.
3. **`scripts/b2-kvm-smoke.py`**: new opt-in env `CTOS_B2_ESR=<hex>|kvm`. With it set, PASS needs the new marker with `iss` equal to the expected value, plus `b2: taken`; a bare `el0: serror`, a different `iss`, `serror-park`, `b2: not taken`, a QMP error or a timeout are FAIL. Unset: unchanged.
4. **User-data `scripts/kvm-recheck-userdata.sh`**: a profile switch (see workflow change), host inventory lines, then the rows below with one ELF.

### Probe rows (one instance, one ELF, three QEMU starts)

`NONCE24` = the low 24 bits of the run's existing per-run nonce.

| Row | QEMU `x-ctos-serror-esr` | Expected guest `iss` (ESR_EL1[24:0]) | Must see | Must not see |
| --- | --- | --- | --- | --- |
| **KN** (nonce) | `0x1000000 \| NONCE24` (IDS=1, impl.-defined bits = nonce) | same value | `el0: virtual serror taken (kvm esr) ec=0x2f iss=<value>`, `b2: taken` | other `iss`, `serror-park`, bare `el0: serror` |
| **KR** (RAS-coded) | `0x000811` (IDS=0, AET=UEO `0b010`, DFSC=`0x11` async SError) | `0x000811` | same marker with `iss=0x000811`, `b2: taken` | as above |
| **KD** (KVM default) | unset (`has_esr=0`) | `0x1000000` (IDS only, KVM's choice [S2]) | same marker with `iss=0x1000000`, `b2: taken` | as above |

Row KD shows the guest reads what KVM chose; rows KN and KR show the value came from this run's host request, not a constant.

**Host inventory (informational, never a pass condition)**, printed as `CTOSB2: ras …` lines:
- `KVM_CHECK_EXTENSION` for cap 158 (`KVM_CAP_ARM_INJECT_SERROR_ESR`) and 178 (`KVM_CAP_ARM_INJECT_EXT_DABT`). Cap 158 = 0 makes the run **FAIL** (reason `no-esr-cap`), because R1 cannot be shown.
- Whether the boot log says the CPU has RAS (`CPU features: detected: RAS Extension Support`).
- Which of the ACPI table **names** `HEST`, `AEST`, `EINJ`, `BERT`, `SDEI` exist under `/sys/firmware/acpi/tables/` (names only; no table contents, no addresses).
- No kernel module, no `/dev/mem`, no write to any RAS register.

**PASS** (the only green): result line `rc=0` with the right commit and nonce; KN, KR and KD each PASS in one attempt of up to 3; `/dev/kvm=present`; `uname` `aarch64`; cap 158 = 1. Result words unchanged from ADR-099 (`PASS | FAIL | INCONCLUSIVE | SKIPPED-no-capacity | REFUSED-active-instance | LEAK | TEARDOWN-SLOW | ABORTED-after-aws-call | NOT-RUN`).

### Workflow change (described only, not implemented here)

In `.github/workflows/kvm-recheck.yml`:
1. New `workflow_dispatch` input `profile`: `b2-serror` (default, today's behaviour, unchanged) or `b2-serror-esr`. The guard refuses any other value.
2. With `b2-serror-esr`: pass `profile` and `NONCE24` to the user-data via the existing `sed` placeholders; the parse step requires the three anchored row PASS lines and cap 158 = 1, and copies the `CTOSB2: ras` lines into the step summary.
3. **Component tag.** The Sep 26 B2-P terms say `component=b2-spike`. The OIDC role's policy (ADR-099, created by DSO) only allows `component=kvm-ci-pilot` on `RunInstances`, `TerminateInstances` and `GetConsoleOutput`. Two ways, sponsor's choice (decision **D2** below):
   - **(a)** keep `kvm-ci-pilot` for this run (no IAM change; the run is still `project=ctos`, `run=<id>`), or
   - **(b)** DSO adds `b2-spike` as a second allowed value in the role policy, and the workflow tags with `b2-spike` when `profile=b2-serror-esr`.
4. Cost settings unchanged: `INSTANCE_TYPE=c7g.metal`, `MAX_PRICE_USD_HR=0.40`, `LIFETIME_MIN=45`, `RUN_COST_CAP_USD=0.50`. No on-demand fallback.
5. ADR-099 says the workflow spec was for **one** supervised pilot; that pilot ran (2026-10-13). A second dispatch needs the sponsor's go (decision **D3**).

### Self-terminate timer and cleanup (unchanged layers from ADR-099)

1. First user-data line: `shutdown -h +45`, with `InstanceInitiatedShutdownBehavior=terminate` and a one-time Spot request with `InstanceInterruptionBehavior=terminate`.
2. The guest powers off after the result line (+90 s linger).
3. The workflow's `always()` teardown terminates by `run=<id>` tag and polls up to 20 min; still running → **LEAK**, still shutting down → **TEARDOWN-SLOW**.
4. Job timeout 75 min. Pre-flight refuses if any `project=ctos` instance is active.
5. Nothing standing: no key pair, no instance profile, no new security group; the volume deletes on termination. After the run, DSO (or the sponsor) confirms with a read-only tag query that no `project=ctos` instance or volume remains.

## Cost estimate

- **Spot price.** A live `DescribeSpotPriceHistory` read for this ADR **failed**: the AWS connector's credentials on the agent side are expired (2026-11-12, about 22:20 CET), and no local AWS keys were used. Last observed `c7g.metal` Spot in eu-north-1: **$0.2476/h** (ADR-085/095, 2026-09-26/28). AWS's public Spot price feed (`spot.js`) was last modified in 2024, so it is stale and not used.
- **On-demand reference** (AWS public price list, eu-north-1, file of 2026-11-10, Linux, shared): `c7g.metal` $2.4755/h, `c6g.metal` $2.336/h, `m6g.metal` $2.624/h, `m7g.metal` $2.7744/h. Not used (no on-demand fallback); shown only to bound the Spot ceiling.
- **Run length** [estimate]: ADR-085 run 4 and ADR-095 each reached PASS in about 7.5–8 min after launch (QEMU build, cargo build, one smoke). This run adds two QEMU starts with the same ELF, about +2 min → **≈ 10–12 min** billed (billing stops at `shutting-down`).
- **Per run:** expected ≈ 0.2 h × $0.25 ≈ **$0.05–0.06**; hard cap `0.40 × (45 + 5) / 60` = **$0.33**. EBS 30 GB gp3 and public IPv4 for under an hour: < $0.01.
- **Milestone total:** one run expected; allow one retry → **≤ $0.70 worst case**, ≈ $0.12 expected. Far under the ~$20 report-first line and inside the `ctos-monthly-5` budget.
- If Spot is above $0.40/h at launch, EC2 refuses (`SpotMaxPriceTooLow`) → **SKIPPED-no-capacity**, no spend. Raising the cap is a sponsor decision, not an agent one.

## Honesty

If HW-VSE passes, say: "On an AWS Graviton3 `c7g.metal` (eu-north-1, Spot, one supervised run, commit `<sha>`), under KVM, the ctos `b2-serror-esr` profile took a **virtual SError** at EL0 three times, and each time `ESR_EL1` held the syndrome the host asked for (a per-run nonce, a RAS-coded value, and KVM's default). The hardware delivered it via `HCR_EL2.VSE`/`VSESR_EL2`; KVM made it pending."

Do **not** say:
- physical SError, hardware error, RAS error handling, or "ctos handles real SErrors";
- that ctos runs on bare-metal Graviton, or that the default image runs under KVM (only the reduced `b2` profile; no GIC/timer/virtio/FAT);
- cpu1 / multi-core under KVM (`-smp 1` only);
- that it is a CI gate or recurring (one supervised dispatch);
- that the "EL0 isolated" (ADR-091 rev 4) or multi-core (ADR-106) sentences change. Any claim change needs a separate written sponsor accept.

Also record the R2 negative as a finding, not a gap to close: on Graviton metal under KVM, a guest cannot raise or receive a physical SError.

## Decisions needed (sponsor, via DSO)

- **D1** Accept HW-VSE (option 1) as the milestone, or pick another row.
- **D2** Component tag: (a) `kvm-ci-pilot` with no IAM change, or (b) DSO adds `b2-spike` to the role policy.
- **D3** Go for one more supervised dispatch of `kvm-recheck` (second use of the ADR-099 path), after the implementation PR merges.
- **D4** Accept the cost basis without a fresh live Spot read (the $0.40/h cap bounds it), or have DSO read current `c7g.metal` Spot in eu-north-1 first.

## Consequences

- This PR: this ADR, a SUMMARY line, a ledger row (Proposed), a MOC line and a research brief.
- Next PR (after D1): patch 0003, the `b2-serror-esr` feature, smoke and user-data changes, and the workflow change above. Its CI cannot exercise KVM; it gets a TCG self-test of the parser and a TCG run of the profile (expected: no KVM, so the ESR rows are not run there).
- Then (after D2–D4 and the merge): one supervised dispatch by the sponsor, an evidence follow-on to this ADR, cleanup check.

## Sources

- [S1] Linux `Documentation/virt/kvm/api.rst`, `KVM_GET/SET_VCPU_EVENTS` (ARM64) and §8.19 `KVM_CAP_ARM_INJECT_SERROR_ESR`: https://docs.kernel.org/virt/kvm/api.html
- [S2] Linux `arch/arm64/kvm/guest.c` (`__kvm_arm_vcpu_set_events`), `arch/arm64/include/asm/kvm_emulate.h` (`kvm_inject_serror`): https://github.com/torvalds/linux/blob/master/arch/arm64/kvm/guest.c
- [S3] QEMU v10.0.0 `target/arm/kvm.c` (`cap_has_inject_serror_esr`, `kvm_put_vcpu_events`): https://github.com/qemu/qemu/blob/v10.0.0/target/arm/kvm.c
- [S4] QEMU v10.0.0 `accel/kvm/kvm-all.c`, `KVM_EXIT_MMIO` (result of `address_space_rw` ignored): https://github.com/qemu/qemu/blob/v10.0.0/accel/kvm/kvm-all.c
- [S5] Arm Neoverse V1 TRM, `ERXPFGCTL_EL1` (access needs `ACTLR_EL3[5]`/`ACTLR_EL2[5]`; `HCR_EL2.TERR`/`SCR_EL3.TERR` traps): https://developer.arm.com/documentation/101427/0102/Register-descriptions/AArch64-system-registers/ERXPFGCTL-EL1--Selected-Error-Pseudo-Fault-Generation-Control-Register--EL1
- [S6] Arm Neoverse V1 TRM, Error injection (CE / DE / UC): https://developer.arm.com/documentation/101427/0102/Functional-description/Reliability--Availability--and-Serviceability/Error-injection
- [S7] Linux `arch/arm64/kvm/sys_regs.c` (`access_ras` → `trap_raz_wi`; `HCR_EL2.TERR`/`TEA` set when the CPU has RAS): https://github.com/torvalds/linux/blob/master/arch/arm64/kvm/sys_regs.c
- [S8] Linux `arch/arm64/kvm/handle_exit.c` (`kvm_handle_guest_serror`): https://github.com/torvalds/linux/blob/master/arch/arm64/kvm/handle_exit.c
- [S9] Linux `arch/arm64/kernel/traps.c` (`arm64_is_fatal_ras_serror`): https://github.com/torvalds/linux/blob/master/arch/arm64/kernel/traps.c
- [S10] Linux `arch/arm64/kvm/mmu.c` (`kvm_handle_guest_sea`, APEI claim): https://github.com/torvalds/linux/blob/master/arch/arm64/kvm/mmu.c
- [S11] AWS, Instances built on the Nitro System (Graviton: UEFI boot with ACPI tables and ACPI PCI hot-plug): https://docs.aws.amazon.com/ec2/latest/instancetypes/ec2-nitro-instances.html
- [S12] AWS, Boot modes (Graviton default UEFI; bare metal is UEFI-only): https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/ami-boot.html
- Kernel sources read at `master` on 2026-11-12; the instance's kernel (Ubuntu 24.04 arm64 AMI) is older, so the run records its `uname` and the cap values rather than assuming them.
