# ADR-091 — DRAFT: umbrella “EL0 isolated” accept (sentence + scope) — pending sponsor accept (M5, sponsor D4)

- Status: **Draft / pending sponsor accept.** Not accepted. Not a claim. This file proposes the exact umbrella sentence and its scope for the sponsor to read (decision **D4**, DSO relay 2026-09-26 ~14:35 CEST). Checklist row **B3** (written sponsor accept, ADR-055 §B row 3) is **not Met** and must not be counted until the sponsor has read this draft and accepted it in writing. Until then the umbrella stays **Planned / non-claim**. Nothing is flipped here: no ledger status change, no roadmap Verified. The flip is M6 and is out of scope for this PR.
- Date: 2026-09-26 (draft)
- Base: stacked on PR [#142](https://github.com/artofdream/ctos/pull/142) (ADR-090, M4).
- Merge: sponsor only ([ADR-002](ADR-002-pr-identity-split.md)). Merging this draft does **not** constitute acceptance; acceptance is a separate written sponsor statement recorded by a follow-up edit (M6).

## Proposed umbrella sentence (verbatim; PROPOSED TEXT, NOT A CLAIM)

> **“On the ctos default smoke machine (QEMU `virt`, `-cpu cortex-a76`, TCG, one core), EL0 is isolated from the kernel at the architectural page-table / exception-level boundary: each EL0 task runs on its own TTBR0 with its own ASID; EL0 faults when it reads or executes kernel memory; PAN is enabled and EL1 faults on EL0 memory; lower-EL IRQ and FIQ are taken while EL0 stands; taken lower-EL SError is evidenced by the ADR-090 evidence class; and the only identity (VA = PA) mapping left in any TTBR0 is the documented EL1-only `_start` stub page (ADR-089). This is not a speculative-execution or side-channel claim, not a real-hardware claim, and not a certification.”**

## Scope: what the sentence covers, and the evidence for each clause

| # | Clause | Evidence (all CI-gated by `scripts/qemu-smoke.sh` unless noted) | ADR |
| --- | --- | --- | --- |
| S1 | Default smoke machine | `.github/workflows/smoke.yml` jobs `QEMU aarch64 smoke (ubuntu-24.04-arm)` / `(ubuntu-24.04)`: distro QEMU `virt -cpu cortex-a76` TCG, no `-smp` (one core) | ADR-079 |
| S2 | Own TTBR0 + ASID per EL0 task | `el0: standing`, `el0: task-ok`, `asid: ok`; EL0 entry without `VMALLE1` (`el0: no-vmalle1`) | ADR-013 / 024 / 039 |
| S3 | EL0 faults on kernel read / execute | `el0: no kernel read` (EL0 load of a kernel-data bait → data abort), `el0: nx kernel` (EL0 fetch of kernel data → instruction abort) | ADR-013 |
| S4 | PAN enabled, EL1-vs-EL0 fault | `pan: present`, `pan: enabled`, `pan: el1-fault` | ADR-079 / 080 |
| S5 | Lower-EL IRQ / FIQ while standing | `el0: irq`, `el0: irq-default`, `el0: fiq` | ADR-040 / 041 / 043 |
| S6 | Taken lower-EL SError | **D1 evidence class, quoted verbatim from [ADR-090](ADR-090-serror-evidence-class.md) §1** (below). The default smoke itself parks (`el0: serror-park`). | ADR-083 / 085 / 090 |
| S7 | Only identity mapping = `_start` stub page | Fail-closed TTBR0 inventory: `ident: inv k=1 u=1 a=1 leaks=0`, `ident: inv-ok allow=stub`, negative probes `ident: inv-neg … caught`, `inv-leak-probe` build caught; leftover/MMIO fault probes `ident: {low,tail,kend,mmio}-fault`. The stub page is EL1 RO+X with `UXN` set, so EL0 cannot fetch it | ADR-087 / 088 / 089 |

### S6: D1 evidence paragraph (verbatim from ADR-090 §1)

> *Taken lower-EL SError while standing at EL0 is evidenced by two sources together. (i) **B1:** the GitHub Actions job `qemu-nmi-pin` (“QEMU NMI pin smoke (taken SError)”) in `.github/workflows/smoke.yml`. It builds QEMU v10.0.0 (`7c949c53e936aa3a658d84ab53bae5cadaa5d59c`) with reviewed local patch `research/qemu-nmi/0001-hw-arm-virt-TYPE_NMI-raise-SError.patch` (TCG: `virt` `TYPE_NMI` → async SError, masked by `PSTATE.A`), boots the default image on `virt -cpu cortex-a76` TCG, issues QMP `stop` → `inject-nmi` → `cont` on the guest's `el0: serror-arm` cue, and with `CTOS_REQUIRE_TAKEN_SERROR=1` fails closed unless the bare `el0: serror` line (printed only by the lower-EL SError vector while the standing-EL0 expectation is armed) appears. It runs on every push and pull request. (ii) **B2-P:** ADR-085 run 4, 2026-09-26 13:26–13:33 CEST, AWS Graviton3 `c7g.metal` (eu-north-1b, Spot), ctos commit `159b178`, QEMU `-accel kvm` with patch 0002 (`inject-nmi` → `KVM_SET_VCPU_EVENTS{serror_pending=1}` → `HCR_EL2.VSE`), opt-in `b2-serror` profile: `el0: serror` + `b2: taken`, fail-closed `b2-kvm-smoke.py` PASS. It is a one-off real-hardware run, not CI. Stock distro QEMU (the default smoke) cannot inject and continues to require `el0: serror-park`.*

The ADR-090 caveats travel with it: B1 is a locally patched emulator; B2-P is historical (at `159b178`, before M1/M2) on a reduced profile; a KVM re-verify ≈ $0.2 is a sponsor option.

## Explicitly **not** in scope (the sentence must never be read as covering these)

- **Speculative execution / side channels.** No KPTI: the TTBR1 kernel mapping stays present (EL1-only) while EL0 runs. No Spectre/Meltdown/cache/timing analysis; QEMU TCG is the wrong lab (threat model: “not side-channel complete”).
- **Real hardware.** Only the emulated default machine, plus the one-off B2-P KVM run on the reduced `b2-serror` profile. No board, no Graviton CI.
- **Multi-core.** One core only. No SMP TLB-shootdown or cross-core isolation story.
- **EL0-to-EL0 (task-vs-task) isolation** beyond the `asid: ok` TLB mile and per-task TTBR0. No probe that one EL0 task cannot read another task's pages.
- **Hypervisor / EL2 / secure world, DMA / IOMMU, physical attacks.** The virtio device can DMA anywhere; there is no SMMU.
- **Certification, formal verification, or “secure OS”.** Not a guest-Linux, container or immutable-OS claim (ADR-029 non-goals stand).
- **Stock-QEMU taken SError.** Stock TCG still parks. The ADR-053 hard-stop for stock virt TCG stands.

## Gaps the sponsor may want closed before accepting (found while drafting; not decided)

- **G1 — Exhaustive EL0-permission walk.** S3 rests on two single-address fault probes plus construction: the kernel descriptor builders (`l1_block`, `l2_block`, `l3_page_flags`) never set `AP[1]` (EL0 access) and always set `UXN` (the only `l1_block` user is Device, also UXN). Only `l3_page_el0_rw` / `l3_page_el0_ro` set `AP[1]`, and those are user pages, checked by code reading on 2026-09-26. No walker today asserts that *every* `AP_EL0` leaf in user TTBR0 is a user page and that *no* TTBR1 leaf carries `AP_EL0`. The identity inventory (S7) checks only VA = PA leaves. Adding such a walk is small (a sibling of `identity_inventory`, marker e.g. `el0: ap-walk ok`), but it is new work.
- **G2 — EL0 write probe.** No probe writes kernel memory from EL0. Write is denied by the same AP encoding that denies read, but it is unprobed. Option: `el0: no kernel write`.
- **G3 — B2-P re-verify on the post-M2 boot path** (≈ $0.2 paid run). Not required by D1.

The sponsor can (a) accept the sentence as drafted with G1–G3 recorded as known limits, (b) require G1/G2 (a further M5b PR) before accepting, or (c) narrow the sentence (e.g. drop “EL0 faults when it reads or executes kernel memory” to “… on the probed kernel addresses”).

## M6 prerequisites (the flip; **not** done here)

1. **B3:** the sponsor's written accept of this sentence and scope (option a/b/c above), recorded verbatim with date and relay path. Merging this PR is not acceptance.
2. **D5:** before the flip, the sponsor/DSO sets branch protection on `main` to **require** `QEMU aarch64 smoke (ubuntu-24.04-arm)`, `QEMU aarch64 smoke (ubuntu-24.04)` and `QEMU NMI pin smoke (taken SError)`. The agent does not change branch protection.
3. #136 → #142 and this PR merged in order, with all three checks green on the merge tip.
4. The flip PR changes ADR-055 §B row 3, ADR-060, the ledger P-SEC-3l row, roadmap, el0.md, SUMMARY and the threat model **only** to the accepted wording, and cites this ADR (status → Accepted).

## Honesty (while Draft)

Say: “ADR-091 is a **draft** of the umbrella sentence and scope, pending the sponsor's written accept; ‘EL0 isolated’ stays Planned / non-claim.” Do **not** quote the proposed sentence as a statement of fact, count B3 as Met, or flip any status.
