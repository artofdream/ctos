# ADR-096 — Task-to-task EL0 isolation probe (task A cannot read or write task B's page)

- Status: Accepted (implementation + evidence). Adds a fail-closed probe that CI gates: an EL0 task on one TTBR0/ASID cannot read or write another EL0 task's private page at the same VA, including right after the other task ran. This is **additional evidence** recorded next to the accepted [ADR-091](ADR-091-el0-isolated-accept-draft.md) sentence. It does **not** change that sentence or its scope. Task-vs-task isolation stays in ADR-091's non-scope list. Any wording that adds it (a possible revision 4) goes to the sponsor in writing first.
- Date: 2026-09-28
- Decision path: sponsor-approved milestone (DSO relay, 2026-09-28 21:21 CEST): "task-to-task isolation probe".
- Base: `main` at `9a1c2bb` (M6 flip, PR #148).
- `_start` (`0x4008_0000`) untouched. No probe maps, reads or writes it.

## Context

ADR-091's non-scope says: "EL0-to-EL0 (task-vs-task) isolation beyond the `asid: ok` TLB mile and per-task TTBR0. No probe that one EL0 task cannot read another task's pages." The `asid: ok` mile (ADR-013) runs at **EL1**: it switches TTBR0 between the kernel L1 tagged ASID 1 and the ASID-B L1 tagged ASID 2 and loads through each. No EL0 code runs in it.

The tree already has a second address space: the ASID-B root (`L1_ASID_B` → `L2_ASID_B` → its own map-window `L3_ASID_B`, ASID 2). Every loaded app uses the user root (`L1_USER`, ASID 1), which shares the kernel's map-window L3.

## Decision

### 1. Two EL0 tasks on two roots (`src/t2t.rs`)

| | Task A | Task B |
| --- | --- | --- |
| TTBR0 | user root `L1_USER`, **ASID 1** (`paging::user_ttbr0()`) | ASID-B root `L1_ASID_B`, **ASID 2** (`paging::asid_b_ttbr0()`) |
| Code | EL0 execute-only page at `0x80002000` (`app-text` slot), shared window L3 | own frame, EL0 execute-only, `nG`, at `0x80002000` in `L3_ASID_B` |
| Private data | round 1: **nothing** at `0x80007000`; round 2: own frame, EL0-RW NX, `nG` | own frame, EL0-RW NX, `nG`, at `0x80007000` (`user-stack` slot) |

- Both tasks run at EL0 through a masked `ERET`. B uses the new `exception::eret_to_el0_masked_ttbr` (an explicit TTBR0 value, ASID included); A uses the existing path. The existing `eret_to_el0_*` helpers now call the same asm with `paging::user_ttbr0()`, so their behaviour does not change.
- A one-shot capture (`exception::arm_el0_capture` / `el0_capture_result`) records ESR, FAR, ELR and `x1` of the task's first lower-EL sync exception (a data abort, or the `BRK #0` that ends the payload) and returns to EL1.
- Payloads are kernel-built: `LDR X1, [X0]` (read) or `MOVZ/MOVK X1, #val; STR X1, [X0]; LDR X1, [X0]` (write), each ending in `BRK #0`, with `X0 = 0x80007000`.
- Frames are written and checked through their TTBR1 alias (PAN blocks EL1 access through the EL0 VA).
- Sentinels: B `0x7a5cb0b0`; A `0x7a5ca0a0`, then `0x7a5ca2a2`; A's attempted store into B's page `0x7a5ceeee`.

### 2. Probe steps (called from `kernel_main_high` after `asid: ok`)

**Round 1 (A has no mapping at the VA):**

1. **Walk:** walk every valid leaf of A's root and B's root for B's frame. A's root must have **no leaf at all** that outputs to B's frame; B's root must have exactly one EL0-reachable leaf for it (control). Marker: `t2t: walk b-frame u-refs=0 u-el0=0 a-el0=1`.
2. **B runs** (ASID 2): stores its sentinel, reloads it (`BRK`, `x1` = sentinel, frame = sentinel). Marker: `t2t: b-own ok asid=2 va=0x80007000 val=0x7a5cb0b0`.
3. **A reads** (ASID 1) the same VA right after B, with **no TLBI of `0x80007000` in between** (A's code-page map/unmap invalidates only `0x80002000`). A must take a lower-EL **translation** fault: EC 0x24, WnR 0, FAR = the VA, ELR = the `LDR`, `x1` still 0. Marker: `t2t: a-read fault asid=1 va=0x80007000 esr=0x92000007 far=0x80007000 ec=0x24 wnr=0 dfsc=trans-l3 got=0x0 after-b no-tlbi`.
4. **A writes** the same VA: translation fault with WnR 1, FAR = the VA, ELR = the `STR`; B's frame still holds B's sentinel. Marker: `t2t: a-write fault asid=1 va=0x80007000 esr=0x92000047 far=0x80007000 ec=0x24 wnr=1 dfsc=trans-l3 b-intact=true`.

**Round 2 (same VA, A gets its own frame):**

5. **Walk:** A's frame is EL0-reachable only in A's root, B's frame only in B's root. Marker: `t2t: walk same-va a-frame u-el0=1 a-el0=0 b-frame u-el0=0 a-el0=1`.
6. B runs again first. Then **A reads** the VA and must see **its own** sentinel, never B's: `t2t: same-va a-sees-own asid=1 got=0x7a5ca0a0 b=0x7a5cb0b0`.
7. **A writes** the VA: the value lands in A's frame only; B's frame is unchanged. `t2t: same-va a-write-own got=0x7a5ca2a2 b-intact=true`.
8. **B reads** the VA after A's trips and sees its own value; A's frame is unchanged. `t2t: same-va b-sees-own asid=2 got=0x7a5cb0b0 a-intact=true`.
9. Summary: `t2t: ok asid-a=1 asid-b=2 va=0x80007000 read,write,same-va`.

Setup checks before step 1: the two TTBR0 values carry ASIDs 1 and 2 and different table bases; both window slots are empty in both roots; B's leaf has `nG`. All probe pages are unmapped (and `TLBI VAAE1`, all ASIDs) before the ADR-092 steady walk. Both VAs are ADR-092 allowlisted slots, so the G1 allowlist does not grow; the steady walk still reports `pages=0 leaks=0`.

### 3. Fail-closed wiring (`scripts/qemu-smoke.sh`)

- The default smoke requires all nine `t2t:` lines above (regex in the script: ESR low bits and table level may vary within "translation fault"). It rejects `t2t: leak`, `t2t: bad` and `t2t: probe missed`, and any task-A line that shows B's sentinel (`got=0x7a5cb0b0`).
- **Leak-probe build:** new feature `t2t-leak-probe` (built together with `reach-leak-probe,write-leak-probe,sysptr-leak-probe` into `target/el0-leak-probe`) maps B's frame into A's root at the same VA (a shared-frame bug). That kernel must print `t2t: leak-probe planted b-frame in a-ttbr0 va=0x80007000`, `t2t: leak walk b-frame u-refs=1 u-el0=1`, `t2t: leak read asid=1 va=0x80007000 got=0x7a5cb0b0` and `t2t: leak write asid=1 va=0x80007000 b=0x7a5ceeee`, and must **not** print `t2t: ok`. The smoke requires this.
- `cargo test` adds `el0_task_cannot_read_or_write_other_task`.
- The three required checks run `scripts/qemu-smoke.sh`, so they enforce all of the above.
- Checker negative test (local, 2026-09-28): the ADR-096 check block from `scripts/qemu-smoke.sh` was run against a passing serial log (it passed) and against five tampered logs: `t2t: ok` deleted, B's sentinel on the task-A same-VA line, `got=` on the read-fault line changed to B's sentinel, `t2t: probe missed` appended, and the real leak-build serial. It exited 1 on each, naming the missing or forbidden line. A sixth log added an extra task-A line with B's sentinel while keeping all required lines; it was rejected by the sentinel guard alone.

## What this proves

On the default smoke machine (QEMU `virt`, `-cpu cortex-a76`, TCG, one core): an EL0 task on the user root (ASID 1) cannot read or write the private page of an EL0 task on the ASID-B root (ASID 2) at the same VA. This holds when A has nothing mapped there (architectural translation fault, B's data never observed, B's frame unchanged), and when A has its own page there (each task sees and changes only its own frame). A's root has no translation of any kind to B's frame. The kernel does not issue a TLB invalidate of that VA between B's trip and A's trip.

## What this does not prove

- **Not part of the ADR-091 sentence.** The accepted sentence is unchanged; task-vs-task stays in its non-scope. A revision 4 would need the sponsor's written accept.
- **Stale-TLB angle on hardware.** QEMU TCG flushes its own software TLB whenever a TTBR0 write changes the ASID (`vmsa_ttbr_write` in QEMU `target/arm/helper.c`). So "no TLBI after B" shows only that the kernel does not rely on a flush it issues itself. It says nothing about how a hardware ASID-tagged TLB would behave. The `asid: ok` mile has the same limit.
- **Two loaded apps.** Both tasks are kernel-built payloads. There is no scheduler that switches between two loaded apps; every loaded app still runs on the one user root (ASID 1), one at a time. The probe checks the page-table and ASID mechanism that such a switch would use, not a multi-app scheduler.
- **One core.** No SMP, no cross-core TLB shootdown.
- **Shared kernel-mapped pages.** The user root shares the kernel's map-window L3 and maps the kernel image text EL1-only; those are covered by the ADR-092 walk (nothing else is EL0-reachable), not by this probe.
- Not side-channel, not real hardware, not DMA/IOMMU, not a certification.

## Consequences

- Honesty ledger row (Verified, scoped as above), threat model **v1.71**, daily brief, SUMMARY and MOC entries.
- `el0.md` points to this ADR as evidence next to the ADR-091 claim. ADR-091 itself is not edited: its sentence, scope table and non-scope list stay exactly as the sponsor accepted them.
