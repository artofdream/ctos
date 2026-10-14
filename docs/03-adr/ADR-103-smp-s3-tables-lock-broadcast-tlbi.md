# ADR-103 — SMP slice S3: type-enforced page-table lock, broadcast TLB invalidation, cross-core unmap probe

- Status: **Accepted for implementation** (sponsor go for S3, 2026-10-13 about 23:37 CEST, effective on the #156 merge at 23:38 CEST; relayed by DSO). Changes **no claim**.
- Date: 2026-10-13
- Follows: [ADR-100](ADR-100-smp-race-story-plan.md) (plan, R1/R2), [ADR-102](ADR-102-smp-s2-secondary-mmu-pan-vbar.md) (S2). Implements ADR-100's slice **S3** exactly as proposed in ADR-102.
- Base: `main` at `bb3f0ee` (#156 merged).
- `_start` untouched; the secondary still enters at `_secondary_entry`. Core 1 still runs **no EL0**.
- $0: the Linux box plus the existing GitHub Actions checks. No cloud agent.

**ADR-091 revision 4 is unchanged** (one core). No SMP safety claim.

## Decision

### 1. `TABLES` is enforced by the type system (R1)

- All 19 page-table statics (`L1`…`L3_HIGH_RAM` and the three bump counters) move into a private module `paging::tables`. Nothing outside it can name them.
- `TABLES` is now `Mutex<Token>`; `Token` has a private field, so only that module can build one. `lock_tables()` masks IRQs (`DAIF.I`), then takes the spin lock, and returns a `TablesGuard` that restores `DAIF` on drop.
- Every function that hands out a writable slot, allocates an L3, clears a root or writes a descriptor takes `&mut TablesGuard`: the 13 slot accessors, 3 allocators, 5 clear helpers, `l3_write`, and (because they call those) `fill_ram_wx`, `fill_user_map`, `install_stack_guards`, `clone_ram_tables_for_high`, `l3_pa_for_ram_block`, `l3_pa_for_user_block`, `unmap_identity_page`, `unmap_user_ram_page`, `set_identity_l3_writable`, `reach_plant_slot`. Forgetting the lock is now a compile error, not a convention. 45 lock sites.
- Taking a table's physical address needs no guard (it writes nothing).

**Limits:** reads still go through `desc_at` / walkers without the lock (racy reads are only a problem once another core writes; core 1 writes no tables). A slot pointer returned under the guard is a raw pointer; it is used only inside the guard's scope by reading, not enforced by a lifetime. Code outside `paging.rs` (for example frame allocation) is not covered.

### 2. Broadcast TLB invalidation at every unmap / tear site (R2)

All TLB maintenance in `paging.rs` goes through two helpers (24 call sites: every unmap, permission downgrade, identity tear, map-window change and the boot `tlbi_all`). Both now do `dsb ishst` → `tlbi vmalle1is` / `tlbi vaae1is` → `dsb ish` → `isb`. The `dsb ishst` before the TLBI is new: it makes the descriptor write visible to other cores' table walkers before the invalidate.

The default single-core build therefore runs the Inner-Shareable forms. On one core they act like the local forms; the default smoke and every existing probe pass unchanged. Not changed: `switch_ttbr0_no_tlbi` (ADR-096 t2t "no TLBI" by design, ASID-tagged) and core 1's own boot-time `tlbi vmalle1` in `_secondary_entry` (its TLB has never been visible to anyone).

### 3. Cross-core unmap probe (opt-in `smp2-probe`, after S2 passes)

1. Core 0 writes a magic word to a dedicated 4 KiB kernel page and maps it in the shared kernel TTBR0 map window (`paging::map_page`, global, under the guard).
2. Core 1 reads it through that VA (its TLB now holds the translation) and signals.
3. Core 0 unmaps it (`paging::unmap_page`: descriptor cleared under the guard, broadcast TLBI) and signals.
4. Core 1 executes one `ldr` from the same VA. Its own sync handler (S2 vectors) records ESR / ELR / FAR and skips the load.
5. Pass: exactly one new exception, `EC=0x25` (data abort, current EL), DFSC a translation fault, `FAR` = the VA, the load's register still holds its sentinel. Core 1 prints `smp: xtlb cpu1 fault ec=0x25 dfsc=trans-l3 far=window own-vectors`; core 0 prints **`smp: xtlb unmap cpu0 → cpu1 fault`**. If core 1 instead reads the old magic without a fault: `smp: xtlb leak`.

### 4. Negative build `local-tlbi-leak-probe`

Same as `smp2-probe` but the two helpers keep the old local-only forms (`tlbi vmalle1`, `tlbi vaae1`). It must print `smp: xtlb leak` and the checker must fail. The smoke also still runs the S1/S2 negatives (`-smp 1`, `smp2-neg-pan`, `smp2-neg-span`, `smp2-neg-vbar`).

## TCG timing (the honesty question)

Under TCG a missing broadcast does not *have* to leave a stale entry: QEMU's per-vCPU soft TLB could be evicted or flushed for other reasons. **Observed on the Linux box (QEMU TCG, `-cpu cortex-a76`, `-smp 2`), 2026-10-13:** the leak build printed `smp: xtlb leak` on **27 of 27** runs and the broadcast build printed `smp: xtlb unmap cpu0 → cpu1 fault` on **27 of 27** runs. So on this QEMU the negative is reliable, and the smoke treats it as required.

This is a property of QEMU's TLB model, not of any CPU. If a future QEMU stops reproducing the leak, the smoke will fail loudly (the negative will not print `smp: xtlb leak`) rather than pass silently. The fallback then is to show the leak on KVM on real arm64 (the ADR-099 pilot path, with sponsor approval), or to have core 1 read the page in a tight loop while core 0 unmaps it and report whether any read succeeded after the unmap completed.

## What this proves

- No code can write page tables, allocate tables or clear roots without holding `TABLES` with IRQs masked: the compiler rejects it.
- On QEMU TCG `-smp 2`: after core 0 unmaps a page with the broadcast TLBI, core 1 faults on its next access (its own vectors handle it); with local-only TLBI, core 1 keeps reading the unmapped page. So the old local TLBI was a real cross-core hole here, and the broadcast form closes it for this case.

## What this does not prove

- Not SMP support or safety; not "EL0 isolated" on two cores. Core 1 runs no EL0, scheduler, heap, drivers or interrupts, and writes no page tables.
- One VA, one page, one unmap direction (core 0 → core 1). Not every tear site is exercised cross-core; the others use the same helper.
- Nothing about ASIDs / non-global entries (S4), lock-free reads of tables, or ordering on real hardware or KVM.
- Nothing about QEMU versions other than the one on the box and the CI runner.

## Proposed S4 (for sponsor go; not started)

ASID allocator with a generation counter (roll-over: `tlbi vmalle1is` + generation bump); a `PerCpu` block via `TPIDR_EL1` (deferred from S2) holding each core's saved kernel TTBR0, EL0 context and expected-fault flags (R4/R5); then run EL0 on core 1 for the first time, kernel-driven, one task per core, and re-run each isolation probe on core 1 with its marker tagged `cpu=1`. The existing leak builds run on core 1 must still be caught. Still no scheduler across cores (S5).

## Consequences

- Default kernel: page-table code now needs the guard (compile-time); TLBI uses the IS forms; behaviour and smoke output unchanged on one core.
- The opt-in CI job builds five ELFs (positive + four negatives); it stays not required and within its 30-minute timeout.
