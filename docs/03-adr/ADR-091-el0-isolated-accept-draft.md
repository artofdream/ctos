# ADR-091 — Umbrella “EL0 isolated” accept (sentence + scope) — Accepted (M5 draft, sponsor D4; B3 met 2026-09-28; M6)

- Status: **Accepted** (2026-09-28, M6). The sponsor accepted the revision-3 sentence and scope in writing: **B3 is met** (record below). “EL0 isolated” is **Verified** only as the sentence below, read together with the non-scope list below. The sentence and the scope are unchanged from revision 3.
- Draft status (2026-09-26 to 2026-09-28, kept for history): **Draft / pending sponsor accept.** Not accepted. Not a claim. This file proposes the exact umbrella sentence and its scope for the sponsor to read (decision **D4**, DSO relay 2026-09-26 ~14:35 CEST). Checklist row **B3** (written sponsor accept, ADR-055 §B row 3) is **not Met** and must not be counted until the sponsor has read this draft and accepted it in writing. Until then the umbrella stays **Planned / non-claim**. Nothing is flipped here: no ledger status change, no roadmap Verified. The flip is M6 and is out of scope for this PR.
- Date: 2026-09-26 (draft); **revision 2: 2026-09-27** after sponsor option (b) (2026-09-27 00:09 CEST: “Close G1 and G2 first … then bring the sentence back to me”). [ADR-092](ADR-092-el0-reach-walk-write-fault.md) closed **G1** and **G2** with CI-gated fail-closed probes and found **G4** (below). **Revision 3: 2026-09-27.** The sponsor skipped the G4 question, so the DSO applied the standing order (close G4 before bringing the sentence back). [ADR-094](ADR-094-syscall-pointer-el0-permission.md) closed **G4** with CI-gated fail-closed probes. The “direct EL0 memory access only” carve-out is dropped. **G3** is the only remaining limit, and it is optional. The sentence below is restated for the sponsor. **Still Draft; B3 still not Met; nothing flipped.**
- **Limits update (2026-09-28, not a revision of the sentence):** the sponsor asked for **G3**. [ADR-095](ADR-095-g3-kvm-recheck.md) re-ran the ADR-085 B2-P procedure on `main` `8107578` on an AWS Graviton3 `c7g.metal` under KVM: **PASS** (`el0: serror` / `b2: taken`, fail-closed smoke, attempt 1). G3 is **done**. The revision-3 sentence and scope are unchanged and need no change. **Still Draft; B3 still not Met; nothing flipped.**
- Base: stacked on PR [#142](https://github.com/artofdream/ctos/pull/142) (ADR-090, M4).
- Merge: sponsor only ([ADR-002](ADR-002-pr-identity-split.md)). Merging this draft does **not** constitute acceptance; acceptance is a separate written sponsor statement recorded by a follow-up edit (M6).

## B3: the sponsor's written accept (verbatim; met 2026-09-28)

- **Quote (verbatim):** “I accept the ADR-091 revision-3 sentence and scope as written. After M6, have ctos propose a short costed list of next milestones.”
- **Date and time:** 2026-09-28 16:41 CEST (Europe/Oslo).
- **Relay path:** sponsor wrote it in the DSO agent chat, DSO relayed it to ctos.
- **Question answered:** “Do you accept the ADR-091 revision-3 sentence and scope as written?” The DSO asked it after showing the sponsor the revision-3 sentence verbatim and the list of what it does not cover.
- **B3 (ADR-055 §B row 3): Met.** The accept covers the revision-3 sentence and the scope and non-scope below exactly as written. This edit does not change either.
- The second sentence of the quote is a request to ctos for after M6. It is not part of the claim.

## Accepted umbrella sentence (verbatim, revision 3; accepted 2026-09-28)

> **“On the ctos default smoke machine (QEMU `virt`, `-cpu cortex-a76`, TCG, one core), EL0 is isolated from the kernel at the architectural page-table / exception-level boundary: each EL0 task runs on its own TTBR0 with its own ASID; EL0 faults when it reads, writes or executes kernel memory; in every translation table (kernel, user and ASID-B TTBR0, and TTBR1) the only EL0-reachable pages are five allowlisted user slots backed by non-kernel frames, checked fail-closed while a user app runs and at rest; PAN is enabled and EL1 faults on EL0 memory; lower-EL IRQ and FIQ are taken while EL0 stands; taken lower-EL SError is evidenced by the ADR-090 evidence class; the only identity (VA = PA) mapping left in any TTBR0 is the documented EL1-only `_start` stub page (ADR-089); and a system call copies from or to an EL0 pointer only if EL0 itself may read (for a copy from EL0) or write (for a copy to EL0) every page in the range. It is not a speculative-execution or side-channel claim, not a real-hardware claim, and not a certification.”**

## Scope: what the sentence covers, and the evidence for each clause

| # | Clause | Evidence (all CI-gated by `scripts/qemu-smoke.sh` unless noted) | ADR |
| --- | --- | --- | --- |
| S1 | Default smoke machine | `.github/workflows/smoke.yml` jobs `QEMU aarch64 smoke (ubuntu-24.04-arm)` / `(ubuntu-24.04)`: distro QEMU `virt -cpu cortex-a76` TCG, no `-smp` (one core) | ADR-079 |
| S2 | Own TTBR0 + ASID per EL0 task | `el0: standing`, `el0: task-ok`, `asid: ok`; EL0 entry without `VMALLE1` (`el0: no-vmalle1`) | ADR-013 / 024 / 039 |
| S3 | EL0 faults on kernel read / write / execute | **Read:** `el0: no kernel read` (EL0 load of the torn identity alias of kernel `.data` → translation fault) and `ttbr1: no el0` (EL0 load of the mapped TTBR1 private page → data abort). **Write (G2, closed):** `el0: write-fault kernel va=0xffffff80… esr=0x9200004f far=<same> ec=0x24 wnr=1 dfsc=perm-l3` (store to kernel `.data` via TTBR1), `el0: write-fault user-ro va=0x80009000 esr=0x9200004f far=0x80009000 ec=0x24 wnr=1 dfsc=perm-l3`, `el0: write-ok kernel,user-ro`; the smoke rejects `el0: write-succeeded` / `el0: write-bad`, and the `write-leak-probe` build must report `el0: write-succeeded`. **Execute:** `el0: nx kernel` (EL0 fetch of kernel data → instruction abort) | ADR-013 / 016 / 092 |
| S8 | Every EL0-reachable page is an allowlisted user slot (G1, closed) | Fail-closed walk of kernel / user / ASID-B TTBR0 **and TTBR1** (EL0-reachable = `AP[1]` set or `UXN` clear; rules: allowlisted VA, pool frame outside heap, not Device, not EL0 W+X, nothing in TTBR1): `el0-reach: live k=3 u=3 a=0 h=0 pages=6 leaks=0` (hello-libctos standing at EL0), `el0-reach: steady k=0 u=0 a=0 h=0 pages=0 leaks=0`, `el0-reach: neg {kernel-pa,va,ttbr1,el0-wx} caught …`, `el0-reach: neg clean`, `el0-reach: ok allow=app-hdr[0x80000000,0x80001000),app-text[0x80002000,0x80003000),crt-stack-pan[0x80003000,0x80004000),user-stack[0x80007000,0x80008000),store-ro[0x80009000,0x8000a000)`; the `reach-leak-probe` build must report `el0-reach: leak … why=kernel-pa` | ADR-092 |
| S4 | PAN enabled, EL1-vs-EL0 fault | `pan: present`, `pan: enabled`, `pan: el1-fault` | ADR-079 / 080 |
| S5 | Lower-EL IRQ / FIQ while standing | `el0: irq`, `el0: irq-default`, `el0: fiq` | ADR-040 / 041 / 043 |
| S6 | Taken lower-EL SError | **D1 evidence class, quoted verbatim from [ADR-090](ADR-090-serror-evidence-class.md) §1** (below). The default smoke itself parks (`el0: serror-park`). | ADR-083 / 085 / 090 |
| S9 | Syscall pointers need EL0 permission on every page (G4, closed) | `el0: sys-ptr control rw-data n=6`; `el0: sys-ptr denied kernel-stub` (`SYS_UART_WRITE` of the `_start` stub page); `el0: sys-ptr denied kernel-window` (kernel-only map-window page); `el0: sys-ptr denied xo-text` and `el0: sys-ptr denied write-ro` (`SYS_NET_MAC` copy_to_user into EL0's own execute-only text / an EL0-RO page, target unchanged); `el0: sys-ptr denied straddle`; seven lines `el0: sys-ptr denied sys=<name> nr=<n> …` (every pointer-taking syscall: 17, 19, 20, 21, 22, 24, 27); `el0: sys-ptr sweep syscalls=7 denied=7`; `el0: sys-ptr ok`. The smoke rejects `el0: sys-ptr leaked` / `bad` / `skip` / `probe missed`. The `sysptr-leak-probe` build must report `el0: sys-ptr leaked kernel-stub n=8` etc. and withhold `ok` | ADR-094 |
| S7 | Only identity mapping = `_start` stub page | Fail-closed TTBR0 inventory: `ident: inv k=1 u=1 a=1 leaks=0`, `ident: inv-ok allow=stub`, negative probes `ident: inv-neg … caught`, `inv-leak-probe` build caught; leftover/MMIO fault probes `ident: {low,tail,kend,mmio}-fault`. The stub page is EL1 RO+X with `UXN` set, so EL0 cannot fetch it | ADR-087 / 088 / 089 |

### S6: D1 evidence paragraph (verbatim from ADR-090 §1)

> *Taken lower-EL SError while standing at EL0 is evidenced by two sources together. (i) **B1:** the GitHub Actions job `qemu-nmi-pin` (“QEMU NMI pin smoke (taken SError)”) in `.github/workflows/smoke.yml`. It builds QEMU v10.0.0 (`7c949c53e936aa3a658d84ab53bae5cadaa5d59c`) with reviewed local patch `research/qemu-nmi/0001-hw-arm-virt-TYPE_NMI-raise-SError.patch` (TCG: `virt` `TYPE_NMI` → async SError, masked by `PSTATE.A`), boots the default image on `virt -cpu cortex-a76` TCG, issues QMP `stop` → `inject-nmi` → `cont` on the guest's `el0: serror-arm` cue, and with `CTOS_REQUIRE_TAKEN_SERROR=1` fails closed unless the bare `el0: serror` line (printed only by the lower-EL SError vector while the standing-EL0 expectation is armed) appears. It runs on every push and pull request. (ii) **B2-P:** ADR-085 run 4, 2026-09-26 13:26–13:33 CEST, AWS Graviton3 `c7g.metal` (eu-north-1b, Spot), ctos commit `159b178`, QEMU `-accel kvm` with patch 0002 (`inject-nmi` → `KVM_SET_VCPU_EVENTS{serror_pending=1}` → `HCR_EL2.VSE`), opt-in `b2-serror` profile: `el0: serror` + `b2: taken`, fail-closed `b2-kvm-smoke.py` PASS. It is a one-off real-hardware run, not CI. Stock distro QEMU (the default smoke) cannot inject and continues to require `el0: serror-park`.*

The ADR-090 caveats travel with it: B1 is a locally patched emulator; B2-P is historical (at `159b178`, before M1/M2) on a reduced profile; a KVM re-verify ≈ $0.2 is a sponsor option. *Update 2026-09-28:* the KVM re-verify was done ([ADR-095](ADR-095-g3-kvm-recheck.md)): the B2-P leg passed again on `main` `8107578` (post-M1/M2, ADR-092, ADR-094), still on the reduced `b2-serror` profile and still a one-off, not CI.

## Explicitly **not** in scope (the sentence must never be read as covering these)

- **Speculative execution / side channels.** No KPTI: the TTBR1 kernel mapping stays present (EL1-only) while EL0 runs. No Spectre/Meltdown/cache/timing analysis; QEMU TCG is the wrong lab (threat model: “not side-channel complete”).
- **Real hardware.** Only the emulated default machine, plus the one-off B2-P KVM run on the reduced `b2-serror` profile. No board, no Graviton CI. (The B2-P run was repeated once on current `main` for G3, [ADR-095](ADR-095-g3-kvm-recheck.md); still reduced profile, still not CI.)
- **Multi-core.** One core only. No SMP TLB-shootdown or cross-core isolation story.
- **EL0-to-EL0 (task-vs-task) isolation** beyond the `asid: ok` TLB mile and per-task TTBR0. No probe that one EL0 task cannot read another task's pages.
- **Hypervisor / EL2 / secure world, DMA / IOMMU, physical attacks.** The virtio device can DMA anywhere; there is no SMMU.
- **Certification, formal verification, or “secure OS”.** Not a guest-Linux, container or immutable-OS claim (ADR-029 non-goals stand).
- **Stock-QEMU taken SError.** Stock TCG still parks. The ADR-053 hard-stop for stock virt TCG stands.
- **Check-to-copy races on SMP.** S9 is a software check on one core. Nothing changes the calling task's user page tables during a syscall. No `AT S1E0R/W`, no `LDTR`/`STTR` copy, no fault-fixup copy (ADR-094).

## Gaps (revision 3, 2026-09-27)

- **G1: exhaustive EL0-permission walk. CLOSED** by [ADR-092](ADR-092-el0-reach-walk-write-fault.md) (S8 markers above). Correction to revision 1: its code-reading note missed `l3_page_el0_exec` (EL0 execute-only, `UXN` clear). The walk covers both `AP[1]` and `UXN`, which supersedes that note.
- **G2: EL0 write probe. CLOSED** by [ADR-092](ADR-092-el0-reach-walk-write-fault.md) (S3 write markers above).
- **G3: B2-P re-verify on the post-M2 boot path. DONE (2026-09-28)** by [ADR-095](ADR-095-g3-kvm-recheck.md): AWS Graviton3 `c7g.metal` Spot, QEMU v10.0.0 + 0001 + 0002 `-accel kvm`, ctos `main` `8107578`: `el0: serror` + `b2: taken`, `b2-kvm-smoke: PASS`, attempt 1, ≈ $0.04. Still the reduced `b2-serror` profile and a one-off, not CI. *Revision-3 text, kept for history:* “(≈ $0.2 paid run). Open, optional, not required by D1. This is the only remaining limit.” **No known limit remains open; B3 (the sponsor's written accept) is the only step left before M6.**
- **G4: syscall copy helpers skip the EL0 permission check. CLOSED** by [ADR-094](ADR-094-syscall-pointer-el0-permission.md) (S9 markers above): `user_range_ok_max` now requires EL0-readable (reads) / EL0-writable (writes) on every page, and the loader maps app text+rodata EL0 read-only + executable. The G1 allowlist is unchanged (five slots). *Revision-2 note, kept for history:* found by ADR-092; documented by code reading; not probed, not fixed at that time. `syscall::user_range_ok_max` accepts any page with a valid leaf in user TTBR0, and `copy_user` then clears PAN. So EL0 could ask `SYS_UART_WRITE` to copy bytes of an EL1-only page mapped in user TTBR0: the `_start` stub page, or an EL1-only map-window page present while the app runs. `copy_to_user` could also write the app's own EL0 execute-only text. Fix: require EL0 read/write permission on the leaf (or `AT S1E0R/W`). The loader must then map app rodata EL0-readable (today it shares the execute-only text page).

The sponsor can:
- (a) accept the revision-3 sentence as it stands, with G3 (optional B2-P re-verify) as the only known limit;
- (b) require G3 (the ≈ $0.2 KVM re-verify) first; *(done 2026-09-28, [ADR-095](ADR-095-g3-kvm-recheck.md): PASS; so (a) and (b) now lead to the same sentence)*
- (c) narrow the sentence further.

Revision-2 options were (a) accept with G3/G4 as limits, (b) fix G4 first, (c) narrow. The sponsor skipped that question. The DSO applied the standing order, which is (b), done in ADR-094.

### History

Revision 1 (2026-09-26) proposed: “… EL0 faults when it reads or executes kernel memory; PAN is enabled … (ADR-089). This is not a speculative-execution or side-channel claim, not a real-hardware claim, and not a certification.” Revision 2 adds “writes”, the all-tables EL0-reachability clause (S8) and the direct-access-only limit (G4). Revision 2's sentence ended its clause list with “… `_start` stub page (ADR-089). This covers direct EL0 memory access only, not data the kernel copies on EL0's behalf in system calls. It is not …”. Revision 3 replaces that carve-out with the S9 syscall-pointer clause (G4 closed by ADR-094).

## M6 prerequisites (all met 2026-09-28; the flip is the M6 PR, branch `isolation/m6-el0-isolated-flip`)

1. **B3:** the sponsor's written accept of the revision-3 sentence and scope (option a/b/c above), recorded verbatim with date and relay path. Merging this PR is not acceptance. **Met:** 2026-09-28 16:41 CEST, recorded above.
2. **D5:** before the flip, the sponsor/DSO sets branch protection on `main` to **require** `QEMU aarch64 smoke (ubuntu-24.04-arm)`, `QEMU aarch64 smoke (ubuntu-24.04)` and `QEMU NMI pin smoke (taken SError)`. The agent does not change branch protection. **Met:** the three checks are required on `main` (set by the sponsor/DSO).
3. The stack merged in order: #136–#138 are already merged (2026-09-26); #139 → #140 → #141 → #142 → #143 (this draft) → #144 (ADR-092) → ADR-094's PR remain, with all three checks green on the merge tip. **Met:** #139–#147 are merged; `main` at `68d4c8f` (merge of #147, ADR-095) is green on all three checks.
4. The flip PR changes ADR-055 §B row 3, ADR-060, the ledger P-SEC-3l row, roadmap, el0.md, SUMMARY and the threat model **only** to the accepted wording, and cites this ADR (status → Accepted). **Done** in the M6 PR.

## Honesty (Accepted, M6)

Say “EL0 isolated” only as the sentence above: quote it verbatim or point to it, and never widen it. Keep its limits next to it: one core; the TCG default smoke machine; not a speculative-execution or side-channel claim; not a real-hardware claim; not a certification; EL0 can read its own code bytes (app text+rodata is EL0 read-only + executable, [ADR-094](ADR-094-syscall-pointer-el0-permission.md)); no multi-core / SMP race story. Do **not** read it as covering anything in the non-scope list above.

*Honesty text while Draft (kept for history):* Say: “ADR-091 is a **draft** of the umbrella sentence and scope, pending the sponsor's written accept; ‘EL0 isolated’ stays Planned / non-claim.” Do **not** quote the proposed sentence as a statement of fact, count B3 as Met, or flip any status.
