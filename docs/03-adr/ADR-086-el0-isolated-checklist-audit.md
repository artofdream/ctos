# ADR-086 — Umbrella “EL0 isolated” checklist audit after B2-P (gap list + milestone plan)

- Status: Accepted (docs / audit). The umbrella “EL0 isolated” (P-SEC-3l) stays **Planned / non-claim**. This ADR audits every [ADR-055](ADR-055-el0-isolated-checklist.md) / [ADR-060](ADR-060-isolation-leftovers-closure-checklist.md) checklist item against evidence gathered **this session** (2026-09-26). It publishes a measured TTBR0 identity inventory, a gap list, and a milestone plan (one milestone per branch/PR). It mints **no** new Verified for the umbrella, taken SError on the accepted smoke machine, or full identity teardown.
- Date: 2026-09-26
- Trigger: sponsor relay (DSO, 2026-09-26 13:55 CEST): item 2 (umbrella EL0 isolated) is unlocked now that B2 has a result ([ADR-085](ADR-085-b2p-graviton-kvm-serror.md)). “Claim it ONLY when every checklist item is Verified by a probe from this session.”
- Base: stacked on PR [#137](https://github.com/artofdream/ctos/pull/137) (`915796f`). #136 / #137 are not merged at the time of writing, and `main` @ `4ea871e` has neither.
- Numbering: next free after ADR-085.

## Context

ADR-055 §B names three Unmet rows that block the umbrella: (1) taken lower-EL SError while standing, (2) full identity teardown including yank `_start`, (3) a written sponsor accept that the umbrella sentence is in scope. ADR-060 is the sponsor one-pager of the same locks. B1 ([ADR-083](ADR-083-b1-qemu-nmi-pin.md)) and B2-P ([ADR-085](ADR-085-b2p-graviton-kvm-serror.md)) have since produced a **taken** `el0: serror`, but only on opt-in paths. The identity tears ADR-018…038 plus ADR-049 landed, while `_start` stays ([ADR-042](ADR-042-isolation-leftover-wrap.md) / [ADR-047](ADR-047-isolation-leftovers-decisions.md) §3, `ident: start-stay`).

This ADR answers: which checklist items are Verified today, which are not, and what exactly is left.

## Evidence gathered this session

| Probe | When (CEST) | What | Result |
| --- | --- | --- | --- |
| Agent-box default smoke | 2026-09-26 13:59–14:04 | `scripts/qemu-smoke.sh` on `915796f`, stock QEMU 10.0.13 (Debian), `virt -cpu cortex-a76` TCG, rustc 1.100.0-nightly `5ceaf6608` | `qemu-smoke: ok`; force-fail exit 1 (fail-closed ok). All §A markers below present. `inject-nmi=>error:machine does not provide NMIs` → `el0: serror-park`. |
| GHA PR run [36239619910](https://github.com/artofdream/ctos/actions/runs/36239619910) (+ push run [36239617837](https://github.com/artofdream/ctos/actions/runs/36239617837)) | 2026-09-26 13:41–13:53 | `smoke.yml` on `915796f`: `ubuntu-24.04-arm` + `ubuntu-24.04` default jobs; opt-in `qemu-nmi-pin` (B1) | All success. Default: park. `qemu-nmi-pin` job 108397549350: `inject-nmi=>ok` → bare `el0: serror` → `qemu-smoke: taken 'el0: serror' present (ADR-083 pinned QEMU)`. |
| B2-P run 4 ([ADR-085](ADR-085-b2p-graviton-kvm-serror.md)) | 2026-09-26 13:26–13:33 | AWS `c7g.metal` KVM, opt-in `b2-serror` profile, one-off paid | `el0: serror` / `b2: taken` / `b2-kvm-smoke: PASS`. Not CI. |
| **Scratch TTBR0 inventory** (not committed to `src/`) | 2026-09-26 ~14:10 | `915796f` + [scratch walker patch](../../research/daily-briefs/2026-09-26-adr-086-ttbr0-inventory-scratch.patch): walks kernel and user TTBR0 L1→L3, coalesces leaves, prints just before `Hello World!` (after every tear). Same QEMU/rustc; smoke rc=0 | [Output](../../research/daily-briefs/2026-09-26-adr-086-ttbr0-inventory.txt). **Five** identity (VA = PA) ranges remain in kernel TTBR0 and **two** in user TTBR0. See §Inventory. |

### Inventory: what is still identity-mapped after every tear

Kernel TTBR0 (the hello boot; the other two boots in the smoke show the same classes, with boundaries shifted by text size):

| # | Range | Size | Attrs | What it is | Status vs “full identity teardown” |
| --- | --- | --- | --- | --- | --- |
| I1 | `0x0000_0000–0x4000_0000` | 1 GiB L1 block | Device-nGnRnE, EL1 RW, PXN+UXN | MMIO identity: GIC `0x0800_0000`, PL011 `0x0900_0000`, virtio-mmio `0x0a00_0000` (all accessed through identity VAs) | **Not torn.** Needs high MMIO alias in TTBR1 first. |
| I2 | `0x4000_0000–0x4008_0000` | 512 KiB | Normal, EL1 RW, XN | RAM below the image (QEMU DTB hole). ctos never reads the DTB (`frame.rs`: “never handed out”) | **Not torn.** Nothing uses it. |
| I3 | `0x4008_0000–0x4008_1000` | 1 page | Normal, EL1 **RO+X** | Boot stub page: `_start` + `exception_vectors` (identity copy; VBAR is high) | **Stays by decision** (`ident: start-stay`, ADR-042 / ADR-047 §3). |
| I4 | `[.ident_tear end, 0x4020_1000)` | ≈ 1.06 MiB (e.g. `0x400f1000–0x40201000`) | Normal, EL1 **RO+X** | Linker padding between the last torn RO section and `.data` (pinned at `0x4020_1000`). No live code. | **Not torn.** The ADR-019/020/025 tears cover `.text` after the stub, `.rodata`, and `.ident_tear` only. |
| I5 | `__kernel_end` page (`0x4025d000`) | 1 page | Normal, EL1 RW, XN | The page between `__fatal_stack_top` / `__kernel_end` and the heap (`ident: heap lo=0x4025e000`) | **Not torn.** Falls between the ADR-037 data tear (`hi=__kernel_end`) and the ADR-038 heap tear. |

User TTBR0 (what EL0 runs on; all EL1-only, `el0=false`): **I3** (boot stub, RO+X) and **I4** (padding tail, RO+X). Kernel `.text`/`.rodata`/`.data`/heap/RAM are already absent from user TTBR0.

Everything else (live `.text` after the stub, `.rodata`, `.ident_tear`, `.data`/`.bss`/stacks, heap, and the frame pool up to `0x4800_0000`) is torn, which matches the Verified tear rows. The inventory is a **scratch probe**: it is Documented evidence for this audit, **not** a committed fail-closed ratchet. That ratchet is milestone M1.

## Checklist audit

Status words: **Verified** = backed by a probe from this session (box and/or CI above). **CI-gated** = `scripts/qemu-smoke.sh` fails closed without the marker on the default GHA jobs. Note that `main` has **no branch protection** (`GET /branches/main/protection` → 404), so “CI-gated” means the sponsor does not merge red. No check is GitHub-required.

### ADR-055 §A — Verified miles (necessary, not sufficient)

| # | Item | Status | Probe / marker (this session) | Recorded | CI-gated? |
| --- | --- | --- | --- | --- | --- |
| A1 | EL0 first mile + NX kernel data | Verified | `el0: ok`, `el0: nx kernel` | Ledger ADR-013 rows; box 14:04 + run 36239619910 | Yes |
| A2 | User TTBR0 omits kernel data | Verified | `el0: no kernel read` (ADR-055 lists `el0: no data`, which is **stale wording**: the code and smoke use `el0: no kernel read`) | Ledger ADR-013 | Yes |
| A3 | Standing EL0 + standing task | Verified | `el0: standing`, `el0: restored`, `el0: task-enter/active/exit/restored`, `el0: task-ok` | Ledger ADR-013/024 | Yes |
| A4 | ASID isolation | Verified | `asid: dual` / `asid: conflict` / `asid: ok` | Ledger ASID row | Yes |
| A5 | TTBR1 private page + EL1 high fetch | Verified | `ttbr1: el1` / `ttbr1: no el0` / `ttbr1: ok`, `ttbr1: el1 exec` / `ttbr1: vbar` | Ledger ADR-016/017 | Yes |
| A6 | Identity tears text / rodata / data / heap / RAM | Verified (as specific tears) | `ident: live`/`range`/`text`, `ident: rodata`(+`-fault`/`-high`), `ident: data`(+…), `ident: heap`(+…), `ident: ram`(+`-fault`/`-high`), `ident: ok`; smoke rejects `ident: heap-stay` / `ident: ram-stay` | Ledger ADR-018…020/025/037/038/049 | Yes |
| A7 | EL0 entry without `TLBI VMALLE1` | Verified | `el0: no-vmalle1` + smoke greps `src/exception.rs` for no `tlbi vmalle1` | Ledger ADR-039 | Yes |
| A8 | Lower-EL IRQ while standing + default I clear | Verified | `el0: irq`, `el0: irq-default` | Ledger ADR-040/041 | Yes |
| A9 | Lower-EL FIQ while standing | Verified | `el0: fiq` | Ledger ADR-043 | Yes |
| A10 | SError **park** honesty (default machine) | Verified | `el0: serror-arm` → `inject-nmi=>error:machine does not provide NMIs` → `el0: serror-park` | Ledger ADR-043/045/053 | Yes |
| A11 | PAN ID field | Verified | `pan: id=` / `pan: present` (cortex-a76) | Ledger ADR-079 | Yes |
| A12 | PAN enable + EL1-vs-EL0 fault | Verified | `pan: enabled`, `pan: el1-fault` | Ledger ADR-080 | Yes |
| A13 | `_start` stay honesty | Verified (honesty; **not** a teardown) | `ident: start-stay lo=0x40080000 hi=0x40081000` | Ledger ADR-042 | Yes |

### ADR-055 §B / ADR-060 — umbrella blockers

| # | Item | Status | Evidence | Honest reading | CI-gated? |
| --- | --- | --- | --- | --- | --- |
| B1 | **Taken lower-EL SError while standing** | **Unmet as worded** (Verified only on two opt-in, non-default paths) | Default machine: park (A10, this session). B1 pin: GHA `qemu-nmi-pin` job 108397549350 (this session) `inject-nmi=>ok` → bare `el0: serror`. B2-P: ADR-085 run 4 (this session) `el0: serror` under KVM. | ADR-060 requires “honest host inject that delivers async SError on the **accepted** smoke machine; serial `el0: serror` under EXPECT”. The accepted smoke machine is still stock distro QEMU `virt` TCG, which parks. **B1** is CI-runnable but uses a locally patched emulator (0001 `TYPE_NMI`→SError). Its job runs only while repo variable `CTOS_BUILD_QEMU_NMI=1`, so it is opt-in, not a gate. **B2-P** is the only real-hardware source (KVM `serror_pending`), but it was one paid run on the b2 profile (no GIC/timer/virtio/FAT) and is **not** CI-runnable without paid metal or a self-hosted arm64 KVM runner. Neither meets the row as worded. It can count only if the sponsor redefines the accepted evidence (decision **D1**). | Row wording implies yes (default smoke + EXPECT). B1 is CI but opt-in. B2-P: no. |
| B2 | **Full identity teardown including yank `_start`** | **Unmet**, and **unmeetable as worded** | Scratch inventory: I1, I2, I4, I5 still identity-mapped in kernel TTBR0; I3 + I4 in user TTBR0. | The row demands yanking `_start`, while ADR-047 §3 decides “never yank `_start`” and the sponsor repeated “never yank `_start` at 0x4008_0000”. As written the row cannot be met. Decision **D2** must redefine it. Independently of `_start`, **four** other identity ranges (I1, I2, I4, I5) are not torn, so even “full teardown except the stub” is Unmet today. | Needs a new fail-closed ratchet (M1). None exists today. |
| B3 | **Written sponsor accept that the umbrella sentence is in scope** | **Unmet** (intent Documented) | DSO relays 2026-09-25 23:44 (“Go with B2 to unlock 1 and 2”) and 2026-09-26 13:55 (“item 2 unlocked … claim ONLY when every item Verified”) | This is **direction**, not an ADR-048/052-style accept that names the exact sentence and its scope (machine, CPU, profile). Decision **D4**. | n/a (docs) |
| B4 | PAN enable (ADR-060 row) | Verified | = A12 | Met (ADR-054 gates 1–4 via ADR-079/080). | Yes |
| B5 | Yank `_start` (ADR-060 row) | Decided: **never** (not a Verified target) | `ident: start-stay` | Conflicts with B2 wording. See D2. | Yes (stay marker) |
| B6 | Guest Linux / containers / immutable-OS marketing | Non-goal | ADR-029 / ADR-036 | Out of scope. Not required for the umbrella, and not unlocked by it. | n/a |

Roadmap P-SEC-3l also paraphrases the umbrella as “standing + PAN + full TTBR1 / identity teardown”: standing (A3) and PAN (A12) are Verified. “Full TTBR1 / identity teardown” = B2 (Unmet).

**Bottom line:** 13 of 13 §A miles plus PAN enable are Verified this session. All three §B blockers are **not** Met. The umbrella stays **Planned / non-claim**.

## Gap list (ordered; identity teardown first)

| Gap | What | Size | Needs sponsor? |
| --- | --- | --- | --- |
| G1 | No committed, fail-closed **identity inventory ratchet**. Today “what is left” is a scratch probe. | Small: walker + allowlist + smoke line + `#[test_case]` | No |
| G2 | I2 low RAM `0x4000_0000–0x4008_0000` (DTB hole) identity RW | Small (unused range) | No |
| G3 | I4 padding tail `[.ident_tear end, 0x4020_1000)` identity RO+X in **kernel and user** TTBR0 | Small (no live code) | No |
| G4 | I5 `__kernel_end` page identity RW | Small | No |
| G5 | I1 MMIO 1 GiB identity Device block: GIC / PL011 / virtio must move to a TTBR1 high alias before the tear | Medium (UART is used pre-MMU and in fault paths; the b2 profile shares the UART) | Scope confirm (D3) |
| G6 | I3 boot-stub page (`_start` + identity vectors) in kernel + user TTBR0 | Small code **if** allowed; policy-blocked | **Yes (D2)** |
| G7 | Taken SError on the *accepted* machine | Policy / CI | **Yes (D1)** |
| G8 | Umbrella accept ADR with the exact sentence | Docs | **Yes (D4)** |
| G9 | Doc drift: ADR-055 §A `el0: no data` ≠ code `el0: no kernel read` | Trivial | No (noted here; fix in M1) |

## Milestone plan (one milestone per branch / PR; all TCG, no AWS)

| M | Branch (proposed) | Scope | Fail-closed markers | Gate |
| --- | --- | --- | --- | --- |
| **M0** (this PR) | `isolation/adr-086-el0-isolated-audit` | This audit + inventory evidence + plan + sponsor decisions. Docs only. | n/a | — |
| M1 | `isolation/adr-087-identity-inventory-ratchet` | Commit the TTBR0 walker as `ident: inv` + tear **G2, G3, G4** (RAM-side leftovers, kernel **and** user TTBR0) + fix G9 | `ident: inv k=<n> u=<n>` + `ident: inv-ok`. Smoke **rejects** any identity leaf outside the allowlist {I1 MMIO block, I3 stub page} (`ident: inv-leak …`). `ident: low` / `ident: tail` / `ident: kend` tears with `-fault` probes. Keeps `ident: start-stay`. | None |
| M2 | `isolation/adr-088-mmio-high-alias` | Map GIC / PL011 / virtio-mmio in TTBR1 (Device, EL1-only), switch base VAs after the high split, tear I1 | `ident: mmio-high`, `ident: mmio-fault` (identity MMIO VA → DABORT), allowlist shrinks to {I3} | D3 (scope). Optional b2-profile KVM re-check is **not** required. If wanted: ≈ $0.2, report cost first. |
| M3 | `isolation/adr-089-boot-stub-policy` | Per **D2**: (a) docs ADR that defines “full identity teardown” as *all identity except the `_start` stub page* (start-stay remains), or (b) code: after the high jump, unmap the identity **mapping** of the stub page (the image and `_start` stay physically at `0x4008_0000`, and QEMU `-kernel` only needs that PA at entry), print `ident: stub-torn`, retire `ident: start-stay` | (b): `ident: stub-torn` / `ident: stub-fault`, allowlist becomes {} → `ident: inv k=0 u=0` | **D2** |
| M4 | `isolation/adr-090-serror-accepted-machine` | Per **D1**: close ADR-055 §B row 1 under the sponsor-chosen evidence class (see D1). A workflow edit (e.g. making `qemu-nmi-pin` unconditional) is pushed from EVO-X2. | Taken `el0: serror` required under the chosen gate | **D1** |
| M5 | `docs/adr-091-umbrella-accept` | ADR-048/052-style sponsor accept: exact sentence, machine/CPU/profile scope, the list of Met rows | n/a | **D4** |
| M6 | `isolation/adr-092-umbrella-flip` | Only if M1–M5 are Met: fresh-session re-probe of every row, ledger/roadmap flip, sponsor merges | All of the above in one run | Sponsor merge (ADR-002) |

ADR numbers in M1–M6 are *proposed*. Each PR takes the next free number at open time.

## Sponsor decisions needed

- **D1 — Which taken-SError evidence counts for the umbrella?** Options: (i) the B1 pinned-QEMU CI job as the accepted smoke machine for this row (make it unconditional; honest caveat: TCG emulator patched locally, not stock); (ii) the B2-P one-off real-hardware run as sufficient (not CI; b2 profile only); (iii) require a CI-gated real-KVM run (self-hosted arm64 KVM runner = security change, or paid metal per gate run ≈ $0.2 each, tagged + self-terminating); (iv) keep taken SError a non-goal → the umbrella is **permanently** non-claim. Agent recommendation: (i)+(ii) together as the evidence class, stated verbatim in the accept ADR, *or* (iv). Not a silent redefinition.
- **D2 — `_start` and “full identity teardown”.** ADR-055 §B row 2 (yank `_start`) contradicts ADR-047 §3 (never yank). Options: (a) the umbrella accepts start-stay as the single documented identity exception; (b) authorize unmapping the stub's identity *mapping* after boot while `_start` stays at `0x4008_0000` (no image move, no `-kernel` change). ADR-047 §3 would then read as “never move or remove `_start`”.
- **D3 — Is device MMIO in scope of identity teardown?** Recommendation: yes (I1 is the largest identity range left).
- **D4 — Umbrella sentence + accept ADR.** Exact wording and scope (e.g. “on QEMU `virt` `-cpu cortex-a76` TCG smoke, plus [D1 evidence]”).
- **D5 — CI gating.** `main` has no branch protection. Should the flip require GitHub-required checks, or is “sponsor does not merge red” enough?

## Honesty

Say: “ADR-086 audited the ADR-055/060 checklist on 2026-09-26: every §A mile and PAN enable are Verified this session. Taken SError is Verified only on opt-in paths (B1 pin, B2-P one-off), not on the accepted smoke machine. Identity teardown still leaves five identity ranges (one is `_start` by decision). No umbrella accept exists. ‘EL0 isolated’ stays Planned / non-claim.”

Do **not** say:

- “EL0 isolated” / “secure OS” / “hardened isolation complete”
- taken lower-EL SError Verified **on the default smoke machine**
- identity mappings fully torn down / `_start` yanked / the kernel moved
- that the scratch inventory is a CI-gated probe (it is Documented research until M1)

## Consequences

- Ledger, roadmap P-SEC-3l, threat model **v1.61**, [el0.md](../framework/el0.md) pointer, SUMMARY, second brain cite this audit.
- ADR-055 / ADR-060 get a follow-on pointer. Their decisions are not rewritten.
- No `src/` change. No AWS. No CloudAgent. Do not self-merge.
