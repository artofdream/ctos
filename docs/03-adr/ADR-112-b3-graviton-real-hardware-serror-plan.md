# ADR-112 — B3: what "real-hardware SError" can honestly mean on Graviton metal (plan; no launch)

- Status: **Proposed (research + plan only).** Nothing here is implemented. **No AWS call was made and no AWS resource was created or changed** to write it. The sponsor and DSO decide; do not self-merge ([ADR-002](ADR-002-pr-identity-split.md)).
- Date: 2026-11-12 (CET)
- Base: main `9ebef21` (after #170). ADR-111 is the last ADR on main and no PR is open, so this is ADR-112.
- Machine: amd64 workstation (docs only; throwaway clone). No cloud, no CloudAgent.
- Asked by: the user, 2026-11-12 22:15 CET: "proceed with next milestone including real-hardware SError".
- Follows: [ADR-085](ADR-085-b2p-graviton-kvm-serror.md) (B2-P, first KVM run), [ADR-095](ADR-095-g3-kvm-recheck.md) (G3 re-run), [ADR-099](ADR-099-kvm-recheck-ci-oidc-sketch.md) (OIDC pilot workflow, the sanctioned launch path), [ADR-108](ADR-108-taken-serror-qemu-spike.md) (TCG spike), [ADR-111](ADR-111-n2-el2-shim-virtual-serror.md) (EL2 shim, virtual SError on stock QEMU).
- Not touched: `src/`, `user/`, `libctos/`, `Cargo.*`, scripts, workflows, `_start` (`0x4008_0000`), the required checks, the ADR-091 rev 4 and ADR-106 sentences.
- Public-identifier rule ([ADR-093](ADR-093-public-identifier-scrub-guard.md)): no account, role, image, subnet, security-group or instance identifier appears here.

## Question

What can a paid Graviton `.metal` run add beyond what ADR-085/095 already showed, and can any option give ctos a **physical** SError on real hardware? Rank the options, pick one, and fix the probe, pass markers, non-claims, cost and teardown **before** anything is launched.

## What is already shown

- **ADR-085 / ADR-095 (B2-P, G3):** on a `c7g.metal` (Graviton3), QEMU `-accel kvm` with patch 0002 set `serror_pending` through `KVM_SET_VCPU_EVENTS`. KVM raised a **virtual SError** (`HCR_EL2.VSE`), and the reduced `b2-serror` profile took it at EL0 (`el0: serror`, `b2: taken`). The ESR was **not** chosen by the host (`serror_has_esr = 0`) and was **not** printed by the guest.
- **ADR-108 / ADR-111 (TCG):** QEMU has no physical SError source. A bad device access is a synchronous abort. The EL2 shim gives a **virtual SError** on stock QEMU.

So "a virtual SError delivered by real silicon to a ctos guest" is already shown once, without a syndrome check.

## Research: what each route can and cannot do

Sources are listed at the end ([S1]–[S10]). The KVM source was read on Linux `master` on 2026-11-12.

| # | Route | What actually happens | Feasible now? | What it would let us claim | What it would not |
| --- | --- | --- | --- | --- | --- |
| A | **ctos as a KVM guest; host injects a virtual SError with a host-chosen ESR** (`KVM_SET_VCPU_EVENTS` with `serror_has_esr = 1`) | KVM writes the ISS into `VSESR_EL2` and sets `HCR_EL2.VSE`; the CPU delivers a **virtual SError** whose `ESR_EL1` carries that ISS [S1][S2]. Needs `KVM_CAP_ARM_INJECT_SERROR_ESR`, which KVM reports only when the host has the RAS extension [S3]. QEMU passes `has_esr`/`esr` only when the cap is present and **silently drops them otherwise** [S4], so the guest must check the value. | Yes. Same workflow, same instance type, small repo changes (below). | Real Graviton silicon delivered a **virtual SError** with a per-run syndrome chosen by the host, and ctos read back exactly that syndrome at EL0. A host RAS / EINJ inventory (read-only). | A physical SError; hardware RAS; the default image; bare metal. |
| B | **Physical error injected on the host** (ACPI APEI **EINJ**, or the RAS pseudo-fault registers `ERXPFGCTL`/`ERXPFGCDN` from a host kernel module) while ctos runs as a guest | EINJ only exists if firmware publishes an EINJ table [S5]; AWS does not document one for Graviton (unknown). `ERXPFG*` injection is an optional RAS feature [S6] and may be trapped by EL3 firmware. If a physical SError then hits while the guest runs, KVM takes it at EL2 and **at most re-injects a virtual SError** into the guest (`kvm_handle_guest_serror` → `kvm_inject_serror`) [S7]. Firmware-first platforms route errors through EL3 / GHES, not to the guest. | Not now. Needs EINJ to exist (probe in A), a written risk evaluation, and a separate sponsor go: it deliberately faults an AWS host, with unknown effect (host crash, hardware-retirement flag). | At most: a physical error on the host was **forwarded** to ctos as a **virtual SError** by KVM. | That ctos took a physical SError. The guest view is always virtual. |
| C | **ctos (guest) triggers a physical SError itself**: unmapped PA, or writing `ERXPFGCTL_EL1` | Under KVM with RAS, `HCR_EL2.TERR` is set and every guest `ERX*` access (including `ERXPFGCTL_EL1`) traps and reads as zero / ignores writes (`access_ras` → `trap_raz_wi`) [S8]. An access outside guest RAM is a stage-2 fault handled by KVM/QEMU as MMIO or turned into a host-injected **synchronous** external abort (`ext_dabt_pending`, `KVM_EXIT_ARM_NISV`) [S1], matching the TCG finding in ADR-108. | No. | Nothing new. | Any SError. |
| D | **ctos bare metal on `.metal`** (no hypervisor; ctos owns EL2/EL1) | Graviton instances boot UEFI only [S9]. ctos is built for QEMU `virt` (PL011 at `0x0900_0000`, `virt` GIC, RAM and `_start` at `0x4008_0000`); the metal platform differs and is unknown. It would need a UEFI loader, a platform port (UART from ACPI SPCR, GIC, RAM map), a custom AMI (new IAM actions: snapshot import / image register), and output through the serial console only. A physical SError would still need a source: `ERXPFG*` at EL1/EL2 (if EL3 does not trap) or EINJ. | No (multi-milestone; changes the boot contract). | Eventually, maybe: ctos took a physical (injected) SError on real hardware. | Anything today. |
| E | **Nested virtualisation**: the ADR-111 EL2 shim runs as a guest hypervisor so ctos owns a virtual EL2 under KVM | Needs `FEAT_NV2` and a host kernel with experimental `kvm-arm.mode=nested` (Linux 6.17+) [S10]. Reports show NV running on Graviton4 (`c8g`, Neoverse V2), with open bugs [S10]. With NV, KVM emulates or delegates the vSError context to the guest hypervisor [S2]. Needs a different instance type (`c8g.metal-24xl`, about 1.7x the price), a newer kernel or AMI and a reboot with a kernel argument. | Not now (experimental host; new AMI; IAM type change). | The full ADR-111 path (shim + full image) on real silicon, under an experimental KVM. | A physical SError (still virtual); stability. |

**Plain answer.** As a KVM guest, ctos can only ever observe a **virtual SError**. KVM keeps physical SErrors at EL2 and only forwards them as virtual ones [S7], and it hides the RAS error-record registers from guests [S8]. A **physical** SError taken by ctos needs ctos on bare metal (D) plus a physical source (B's mechanisms). Neither is available today. "Real-hardware SError" for this milestone therefore means a **virtual SError delivered by real Graviton silicon**, now with a host-chosen syndrome the guest must read back. It is the honest next step up from ADR-085/095.

## Options ranked

1. **A: host-chosen-syndrome virtual SError + read-only RAS/EINJ inventory (chosen).** Cheapest, uses the sanctioned workflow and the same instance type, gives a fail-closed per-run value, and answers the "is EINJ even there?" question for B without injecting anything.
2. **E: nested virt on Graviton4.** Next, if the sponsor wants the full image and the EL2 shim on silicon. Still a virtual SError.
3. **B: host EINJ / ERXPFG injection.** Only if A's inventory shows EINJ, and only with a written risk evaluation and a separate sponsor go.
4. **D: bare-metal ctos.** A separate, multi-ADR project.
5. **C: guest-triggered physical SError.** Rejected: KVM blocks it by design.

## Decision (proposed): B3 = option A

### Probe (one instance, one run, three rows, plus an inventory)

The run reuses `c7g.metal`, the reduced `b2-serror` profile and the QEMU v10.0.0 build with patches 0001 and 0002, adding a new patch 0003. The run nonce is the existing workflow nonce. `N16` is its low 16 bits, or `0x00c7` if those are zero.

**Inventory** (read-only, printed before any row; table names and yes/no only, never contents):
- `CTOSB2: ras host-ext=yes|no`: whether `dmesg` has the kernel's "RAS Extension Support" CPU-feature line [S3].
- `CTOSB2: ras kvm-cap-serror-esr=<n>`: `KVM_CHECK_EXTENSION(KVM_CAP_ARM_INJECT_SERROR_ESR = 158)` on `/dev/kvm`.
- `CTOSB2: ras acpi EINJ=yes|no HEST=yes|no AEST=yes|no BERT=yes|no SDEI=yes|no`: whether each file exists under `/sys/firmware/acpi/tables/`.
- `CTOSB2: ras kconfig APEI_EINJ=y|m|n`: read from the running kernel's config.

**Rows** (each a separate QEMU boot, through the fail-closed `b2-kvm-smoke.py`):

| Row | Host action | Must see | Must not see |
| --- | --- | --- | --- |
| N (negative) | No inject at the `el0: serror-arm` cue | `el0: serror-park`, `b2: not taken` | bare `el0: serror`, `b2: taken` |
| D (default) | Inject, `serror_has_esr = 0` (today's B2-P path) | `el0: serror`, `b2: taken`, `b2: esr=0xbf000000` (KVM's default ISS is IDS/ISV only [S2]) | `serror-park` |
| E (chosen syndrome) | Inject, `serror_has_esr = 1`, `serror_esr = 0x0100_0000 \| N16` | `el0: serror`, `b2: taken`, `b2: esr=0x(bf00_0000 \| N16)`, exactly | any other `b2: esr=` value |

`ESR_EL1` for a **virtual SError** is EC `0x2f` (`0xbc00_0000`) + IL (`0x0200_0000`) + the ISS from `VSESR_EL2`. With IDS (bit 24) set, that is `0xbf00_0000 | N16`. KVM rejects ISS bits above bit 24 (`-EINVAL`) [S1][S2].

**Pass (fail-closed):** the workflow reports PASS only if every one of these holds:
1. The existing PASS lines are present: `result smoke-rc=0 sha=<sha> nonce=<nonce>`, `kvm-api 12`, the `uname … aarch64` line, and the smoke's PASS line.
2. `CTOSB2: ras kvm-cap-serror-esr=1`. A `0` makes the run **INCONCLUSIVE**, because QEMU would silently drop the ESR [S4].
3. All three rows are ok, with row E's ESR equal to the nonce-derived value.

The inventory values are recorded and are not pass conditions, except for the cap.

### Repo changes this needs (described only; a separate implementation PR after this ADR is accepted)

- **W1** `research/qemu-nmi/0003-…patch` (applied by `build-qemu-b2.sh` after 0002): in the KVM branch of `virt_nmi`, if `CTOS_B2_SERROR_ESR` is set to a hex value with no bits above bit 24, set `serror.has_esr = 1` and `serror.esr` to it. Otherwise behave as today. TCG is unchanged.
- **W2** `src/` (only under the `b2-serror` feature): the lower-EL SError handler keeps `ESR_EL1`; `observe_b2_serror` prints `b2: esr=0x%08x` after `b2: taken`. The default build and the other features are byte-for-byte unchanged, and `_start` is unchanged.
- **W3** `scripts/b2-kvm-smoke.py`: `CTOS_B2_EXPECT_ESR` (exact match, else FAIL `esr-mismatch`) and `CTOS_B2_NO_INJECT=1` (row N: never sends `inject-nmi`; passes only on the not-taken markers).
- **W4** `scripts/kvm-recheck-userdata.sh`: the inventory lines, rows N/D/E in that order, and `rows=N:ok,D:ok,E:ok` added to the result line.
- **W5** `.github/workflows/kvm-recheck.yml`:
  - The PASS parse requires items 1–3 above.
  - The acknowledgement string becomes a new per-milestone value (`supervised-b3-run`), because the ADR-099 acknowledgement was for one pilot run.
  - The `component` tag value follows the sponsor's choice below, in the launch tags and in both teardown filters.
  - Cost knobs are unchanged: `MAX_PRICE_USD_HR 0.40`, `LIFETIME_MIN 45`, `RUN_COST_CAP_USD 0.50`.
- **IAM (DSO/sponsor side, not repo text):** a role change is needed **only** if the sponsor picks `component=b2-spike` or `c6g.metal`. The role's tag condition currently requires `component=kvm-ci-pilot`, and its instance-type condition requires `c7g.metal`. Nothing else changes: no new action and no `iam:PassRole`.

### Instance type, tags and cost

**Spot price, eu-north-1** (the workflow's region, Linux). Read from the public EC2 Spot pricing feed (`website.spot.ec2.aws.a2z.com/spot.json`, regional figure). The feed was updated 2026-11-12 22:16 CET and read at 22:18 CET.

| Type | CPU | $/h |
| --- | --- | --- |
| `c6g.metal` | Graviton2 (Neoverse N1) | **0.2336** (cheapest Graviton metal) |
| `c7g.metal` | Graviton3 (Neoverse V1) | 0.2476 (same as ADR-085/095's `DescribeSpotPriceHistory` in eu-north-1b) |
| `m6g.metal` | Graviton2 | 0.2624 |
| `m7g.metal` | Graviton3 | 0.2879 |
| `c8g.metal-24xl` | Graviton4 (option E only) | 0.4085 |

The run re-reads the price at launch: the EC2 max price stays `0.40`, so a price above that is `SKIPPED-no-capacity`, not a silent overspend.

- **Recommendation:** keep **`c7g.metal`**. The role allows only that type, and it is the same silicon as the ADR-085/095 evidence. `c6g.metal` would save about **$0.005 per run** (0.014 $/h over about 20 min) but needs an IAM change. The B2-P terms say "cheapest", so this is a **sponsor choice**, listed below.
- **Run length:** ADR-095 took 8.1 min to self-poweroff and about 15.5 min to `terminated`. B3 adds two smoke rows (at most 90 s each) and an inventory (seconds). Estimate: **about 11 min running, about 20 min billed** including the metal shutdown.
- **Cost per run:**
  - Expected: 20/60 h × $0.2476 ≈ **$0.08**, plus gp3 30 GB and one public IPv4 for about 20 min (< $0.01). About **$0.09**.
  - Hard cap: the guard's `0.40 × (45 + 5)/60` = **$0.33**. The worst case including a slow 20-min teardown is 0.40 × 65/60 ≈ **$0.43**.
- **Budget asked:** one run, plus at most two re-runs if a run ends `SKIPPED-no-capacity`, INCONCLUSIVE or FAIL for infrastructure reasons. That is about **$0.09 expected** and at most **about $1.30**, far under the about $20 gate, so no separate cost approval is triggered. GitHub Actions minutes are free for this public repo.
- **Tags:** `project=ctos` on everything. `component` is `b2-spike` per the B2-P terms (needs the IAM tag-value change) **or** `kvm-ci-pilot` (no IAM change), per the sponsor's choice. A `run=<run id>` tag is added as today.

### Self-terminate timer and cleanup (unchanged layers from ADR-099, all already in the workflow)

1. The first line of the user data is `shutdown -h +45`. With `instance-initiated-shutdown-behavior terminate`, poweroff means termination.
2. One-time Spot with `InstanceInterruptionBehavior=terminate` and a max price.
3. An `always()` teardown terminates every instance with this run's tag and waits up to 20 min. It reports LEAK or TEARDOWN-SLOW (red), never green.
4. Pre-flight refuses to launch if any `project=ctos` instance is active.
5. The root EBS has `DeleteOnTermination=true`. There is no key pair, no instance profile and no inbound rule. Results leave as `CTOSB2:` console lines only.

**Cleanup check after the run** (DSO, read-only):
- No `project=ctos` instance is in a non-terminated state.
- No `project=ctos` volume is left. The role has no `DescribeVolumes`, so this is a console or `DescribeVolumes` read by DSO.
- The Spot request is closed.

## Honesty

**If B3 passes, say:** "B3 (ADR-112): on an AWS Graviton3 `c7g.metal` (eu-north-1, Spot, one supervised run), the KVM host set a per-run SError syndrome with `KVM_SET_VCPU_EVENTS` (`serror_has_esr`). The CPU delivered it as a **virtual SError** (`HCR_EL2.VSE` + `VSESR_EL2`), and the reduced ctos `b2-serror` profile, standing at EL0, took it and read back exactly that syndrome in `ESR_EL1`. Without the inject it parks. Host inventory: RAS extension <yes/no>, ESR-inject cap 1, ACPI EINJ <yes/no>."

**Do not say:**
- that ctos took a **physical** SError, or a hardware RAS error;
- that ctos runs on real hardware without a hypervisor, or that the default image boots under KVM (only the reduced profile runs: no GIC, timer, virtio or FAT);
- that EINJ or `ERXPFG*` injection works (A only lists tables; nothing is injected);
- that a guest can reach the RAS registers (KVM makes them RAZ/WI [S8]);
- anything about cpu1 / SMP, or a CI gate (this is a supervised one-off);
- that the ADR-091 rev 4 or ADR-106 sentence changes. A claim change needs a separate written sponsor accept.

## Approvals and actions needed before any launch

1. **Sponsor review of this ADR** (choose option A and accept the non-claims). No self-merge.
2. **Sponsor choices:** the `component` tag value (`b2-spike`, which needs a DSO-side role tag-condition change, or `kvm-ci-pilot`, which needs none), and the instance type (`c7g.metal` recommended, or `c6g.metal`, which needs a role change).
3. **The implementation PR (W1–W5)**, reviewed and merged to `main`. The workflow only tests commits on `main`.
4. **Re-check the ADR-099 conditions at dispatch time** (DSO): CloudTrail on, `main` protection enforced for admins, the `kvm-pilot` environment with required reviewers, and the repository variables still set.
5. **A sponsor go for one supervised dispatch** with the new acknowledgement string, plus the environment reviewer's approval. Cost: about $0.09 expected, at most $0.43 per run and at most about $1.30 total. That is under the $20 gate.

## Consequences

- New: this ADR, a ledger row (**Planned**), SUMMARY and moc lines, and a brief.
- No code, script, workflow, Dockerfile or AWS change in this PR.

## Implementation (W1–W5; not run)

Added 2026-11-12 (CET) by the implementation PR after this ADR merged (#172). Status stays **Proposed**, and the ledger row stays **Planned**: nothing was launched and no AWS call or workflow dispatch was made.

- **W1** `research/qemu-nmi/0003-hw-arm-virt-KVM-serror-host-chosen-esr.patch`, applied by `scripts/build-qemu-b2.sh` after 0002. With `CTOS_B2_SERROR_ESR` set to a hex ISS (bits 0–24 only), the KVM branch of `virt_nmi` sets `serror.has_esr = 1` and `serror.esr`. If KVM lacks `KVM_CAP_ARM_INJECT_SERROR_ESR`, or the value is malformed, it injects nothing, prints an error and fails the `inject-nmi` QMP command. Unset: unchanged from 0002. TCG: unchanged.
- **W2** `b2-serror` only: the lower-EL SError handler keeps `ESR_EL1`, and the probe prints `b2: esr=0x%08x` after `b2: taken`. In the default build, `.text`, `.data` and `.bss` are byte-identical to main. `.rodata` differs only in 16 bytes, which are panic-location line numbers shifted by the added source lines. `_start` is unchanged.
- **W3** `scripts/b2-kvm-smoke.py`: `CTOS_B2_EXPECT_ESR` (exactly one `b2: esr=` line, an exact match, else `FAIL esr-mismatch`), `CTOS_B2_NO_INJECT=1` (row N), and a refused inject is `FAIL inject-error`. Unit tests: `scripts/test_b2_kvm_smoke.py`.
- **W4** `scripts/kvm-recheck-userdata.sh`: the read-only inventory (`ras host-ext`, the boot-log RAS line, `ras kvm-cap-serror-esr`, `ras kvm-cap-ext-dabt`, `ras acpi EINJ/HEST/AEST/BERT/SDEI` names, `ras kconfig APEI_EINJ`), then rows N, D and E (up to two boots each), with `rows=N:ok,D:ok,E:ok` in the result line.
- **W5** `.github/workflows/kvm-recheck.yml`: PASS needs the result line with `rows=N:ok,D:ok,E:ok`, each row's PASS line, row E's ESR derived from the nonce inside the workflow, and no `esr-mismatch` on any attempt. `kvm-cap-serror-esr` other than `1` is **INCONCLUSIVE**. The acknowledgement is `supervised-b3-run`. `component=b2-spike` is used in the launch tags and in both teardown filters. The cost knobs, instance type and cleanup layers are unchanged.

The local probes (TCG, unit tests, lint) are listed in the PR. Only a supervised KVM run on Graviton can show rows D and E: under TCG, QEMU emulates a physical SError with `ESR_EL1 = 0xbe000000`, and patch 0003 does nothing there.

## Sources

- [S1] Linux, `Documentation/virt/kvm/api.rst`, `KVM_GET/SET_VCPU_EVENTS` (arm64): `serror_has_esr`, ISS in `serror_esr`, `-EINVAL` above the ISS, `ext_dabt_pending`. https://docs.kernel.org/virt/kvm/api.html
- [S2] Linux, `arch/arm64/kvm/inject_fault.c` (`kvm_inject_serror_esr`: `vcpu_set_vsesr`, `HCR_EL2_VSE`; NV delegation) and `arch/arm64/include/asm/kvm_emulate.h` (`kvm_inject_serror` uses `ESR_ELx_ISV`); `arch/arm64/kvm/guest.c` (`__kvm_arm_vcpu_set_events`). https://github.com/torvalds/linux/tree/master/arch/arm64/kvm
- [S3] Linux, `arch/arm64/kvm/arm.c` (`KVM_CAP_ARM_INJECT_SERROR_ESR` = `ARM64_HAS_RAS_EXTN`); `arch/arm64/kernel/cpufeature.c` ("RAS Extension Support"); `include/uapi/linux/kvm.h` (cap 158).
- [S4] QEMU v10.0.0, `target/arm/kvm.c` `kvm_put_vcpu_events` (passes `has_esr`/`esr` only if `cap_has_inject_serror_esr`). https://gitlab.com/qemu-project/qemu/-/blob/v10.0.0/target/arm/kvm.c
- [S5] Linux, "APEI Error INJection". https://docs.kernel.org/firmware-guide/acpi/apei/einj.html ; ACPI 6.4 §18.6. https://uefi.org/htmlspecs/ACPI_Spec_6_4_html/18_ACPI_Platform_Error_Interfaces/error-injection.html
- [S6] Arm, "Learn the architecture: RAS Hardware Guide", Error injection (`ERR<n>PFGCTL`, `PFGCDN`). https://support.arm.com/documentation/110247/101/RAS-system-control-and-recording/Error-injection
- [S7] Linux, `arch/arm64/kvm/handle_exit.c` (`kvm_handle_guest_serror`: a physical SError taken while a guest runs is handled at EL2 and at most re-injected as a virtual SError).
- [S8] Linux, `arch/arm64/kvm/sys_regs.c` (`HCR_EL2_TERR` when RAS; `access_ras` → `trap_raz_wi` for `ERX*`, including `ERXPFGCTL_EL1`).
- [S9] AWS, EC2 boot modes (Graviton instances: UEFI only). https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/ami-boot.html ; AWS Graviton technical guide (Graviton2 = Neoverse N1 / Armv8.2, Graviton3 = V1 / Armv8.4, Graviton4 = V2 / Armv9.0). https://aws.github.io/graviton/
- [S10] M. Zyngier, "Re: KVM progress with FEAT_NV2" (NV2 experimental since 6.17). https://lore-kernel.gnuweeb.org/kvmarm/877bjt3lv7.wl-maz@kernel.org/ ; linux-arm-kernel list, "Nested virt bug on KVM arm64 when SVE is enabled" (Graviton4 / Neoverse V2 host on Linux 6.17.3; search the list archive by that subject) ; Arm Learning Path, nested virtualization (needs FEAT_NV2, bare metal). https://learn.arm.com/learning-paths/servers-and-cloud-computing/nested-virtualization/
