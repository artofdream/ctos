# ADR-104 — SMP slice S4: PerCpu via TPIDR_EL1, ASID allocator with generations, first EL0 on core 1

- Status: **Accepted for implementation** (sponsor "Go on S4", relayed by DSO on 2026-10-19 at 17:31 CEST; the stop-all hold is lifted for this item only). Changes **no claim**.
- Date: 2026-10-19
- Follows: [ADR-100](ADR-100-smp-race-story-plan.md) (plan, R3/R4/R5), [ADR-102](ADR-102-smp-s2-secondary-mmu-pan-vbar.md) (S2), [ADR-103](ADR-103-smp-s3-tables-lock-broadcast-tlbi.md) (S3). Implements the "Proposed S4" section of ADR-103 exactly.
- Base: `main` at `3234668` (#158 merged).
- `_start` untouched; core 1 still enters at `_secondary_entry`. The default single-core build keeps its behaviour and its full smoke.
- $0: the Linux box plus the existing GitHub Actions checks. No cloud agent. No workflow file changed.

**ADR-091 revision 4 is unchanged** (one core). No multi-core safety claim. Any multi-core sentence is S5 and needs a separate sponsor accept.

## Decision

### 1. `PerCpu` via `TPIDR_EL1` (R4, R5)

- New `src/percpu.rs`: one 64-byte-aligned `PerCpu` block per core (`NCPU = 2`): saved kernel TTBR0 at offset 0, core index, last ASID generation seen. `TPIDR_EL1` holds the block's TTBR1 high VA.
- `TPIDR_EL1` used to hold the saved kernel TTBR0 directly. The four lower-EL trampolines (sync, IRQ, FIQ, SError) now do `mrs x2, tpidr_el1; cbz; ldr x2, [x2]; cbz; msr ttbr0_el1, x2` (one extra load).
- Core 0 installs its block in `kernel_main_high` **after** every identity tear, before the first EL0 entry. Installing it earlier failed: the ADR-037 pointer rewrite covers `[__data_start, __kernel_end)`, and the saved TTBR0 is the kernel L1's PA, which lies in `.bss` and looks like an identity pointer, so it was rewritten to a high VA. The boot then faulted on the first EL0 return. Until the install, `TPIDR_EL1` is 0 and the trampolines skip the restore (no EL0 runs before then).
- `percpu::Banked<T>` gives each core its own copy of a static and derefs to the reading core's copy. Banked: the 11 EL0-context statics in `el0.rs` (`ENTERED`, `ACTIVE`, `USER_PC`, `USER_SP`, `USER_TTBR`, `KIND`, …) and the 60 expected-fault / caught / EL0-return statics in `exception.rs` (`EXPECT_*`, `*_CAUGHT`, `EL0_CONT`, `EL0_KSP`, …). Code that uses them is unchanged. On one core only slot 0 is used.

### 2. ASID allocator with a generation counter (R3), `smp2-probe` only

New `src/asidgen.rs`. 8-bit ASIDs; 0 (kernel), 1 and 2 (the fixed ADR-013 / ADR-096 probe ASIDs) are never issued, so 3..=255 (253 per generation). A task holds `(generation, asid)`. Allocation runs under a spin lock. ASIDs are never freed one at a time. A retired task's tables may be torn down without a TLBI, because its ASID is not reissued until the next rollover.

**Rollover** (bitmap exhausted): generation + 1, clear the bitmap, keep each core's **running** ASID reserved into the new generation (so a task live on another core never shares its ASID with a new task), then `dsb ishst; tlbi vmalle1is; dsb ish; isb` before any ASID is reissued. `switch_to` revalidates a slot: same generation, keep it; older generation but still this core's reserved running ASID, keep it under the new generation; otherwise allocate a fresh one. Each core records the generation it last switched under in `PerCpu::asid_gen_seen`.

Not wired into the default build: the default single-core kernel still uses the fixed ASIDs 1 and 2.

### 3. Core 1 in the kernel's normal mode; the first EL0 on core 1 (`smp2-probe`)

After S1–S3 pass, core 0 runs `vfs::init`, `virtio::init`, `init_net`, `fat::init`, `sched::init` and `gic::init` (distributor and its own CPU interface; no timer on core 0), then signals core 1 and waits without printing. Core 1:

1. installs `PerCpu[1]` in `TPIDR_EL1`;
2. takes core 0's `SCTLR_EL1.SPAN` (so a build that undoes the PAN fix behaves on core 1 as on core 0);
3. sets `VBAR_EL1` to the **kernel** vector table (no longer its S2 table), `SP_EL1` to its own 16 KiB exception stack, and `SP_EL0` to its own 64 KiB thread stack; then `SPSel = 0` (EL1t, like core 0's kernel threads);
4. enables its own banked GIC CPU interface, timer PPI and CNTP (`timer::init`);
5. runs the single-core boot's EL0 isolation probes, in the same order, **on core 1**: `el0` (SVC, NX, read / write faults, standing + IRQ + FIQ + SError-arm), `svc`, `sys-ptr`, `libctos`, `loader` (also arms the live reach and PAN checks), `pan-held`, `asid`, `t2t`, `xo`, `ttbr1`, `ident` (identity inventory), `el0-reach`, `pan`. After each one it prints `smp: cpu1 probe <name> ok cpu=1` or `… missed cpu=1`; the probes print their own markers as on one core.

One task per core: core 0 runs no EL0 in this build. The standing loader task (`observe_standing_task`) and the non-isolation samples (FAT / net demos) are not part of S4.

### 4. ASID-generation probes (both cores)

1. **Concurrent allocation:** both cores allocate 150 ASIDs at the same time (300 > 253, so at least one rollover). Core 0 checks that every `(generation, asid)` pair is unique, every ASID is in 3..=255, and that at least one rollover happened: `smp: asidgen alloc cpu0+cpu1 n=300 unique range=3..=255 gens=2 rollovers=1`.
2. **Reissue after rollover:** core 0 maps a magic page (non-global) in the kernel map window. On core 1, task T1 gets ASID *x* (generation *N*), switches TTBR0 to the kernel L1 tagged *x*, and reads the page (core 1's TLB now holds an *x*-tagged entry). T1 exits. Core 0 then clears the descriptor **without any TLBI** (the generation rule) and allocates until *x* is issued again in a later generation. That forces a rollover, which flushes with `tlbi vmalle1is`. Core 1 runs task T2 with that slot (same TTBR0 value) and does one `ldr` from the page. Pass: an EL1 translation fault on the kernel vectors, caught by a per-core expected-fault flag: `smp: asidgen cpu1 fault ec=0x25 dfsc=trans-l3 gen-seen=3 cpu=1`, then `smp: asidgen reissue cpu0 rollover → cpu1 fault`.
3. Core 0 prints `smp: ok cpu1 s4 percpu=tpidr el0=cpu1 probes=13 asidgen=unique,rollover-is` only if core 1 entered kernel mode, all 13 probes passed on core 1, the allocation was unique, and the reissue faulted.

### 5. Negative build (one ELF, all leaks on core 1)

`smp2-probe` + the new `asid-rollover-local-probe` (rollover flush is the local `tlbi vmalle1`) + the existing leak features: `inv-leak-probe`, `reach-leak-probe`, `write-leak-probe`, `sysptr-leak-probe`, `t2t-leak-probe`, `xo-leak-probe`, `pan-keep-leak-probe`. The smoke runs the **same** log checks as the single-core leak builds (factored into `inv_leak_check` / `el0_leak_check`; the only change is that the xo miss line is core 1's `smp: cpu1 probe xo missed cpu=1` instead of `main.rs`'s `xo: probe missed`). It also requires:

- `smp: cpu1 probe {el0,sys-ptr,pan-held,t2t,xo,ident,el0-reach} missed cpu=1`, `smp: cpu1 probes missed failed=7 n=13`;
- `… reissued (flush=local)`, `smp: asidgen cpu1 read the retired page under the reissued asid cpu=1`, `smp: asidgen leak`;
- no `smp: ok cpu1 s4`, and the S1–S4 checker must fail.

### 6. Smoke

- `scripts/qemu-serial-inject.py`: new opt-in `CTOS_QEMU_SMP` (adds `-smp N`) and `CTOS_QEMU_STOP_AT` (stop at a serial string). Both absent means the default runs exactly as before.
- `smp2_check` additionally requires all S4 lines, the 13 probe tags exactly once each, and the probes' own ok markers. The console-lock allow-list covers the new `smp:` lines.
- The S4 positive run and the S4 leak run use the inject script (virtio-blk FAT image, virtio-net) with `-smp 2`. The `-smp 1`, `smp2-neg-*` and `local-tlbi-leak-probe` runs are unchanged (they stop before S4).
- The opt-in CI job (`CTOS_SMP2_ONLY=1`) builds one more ELF (six in total). Every `cargo build` already publishes the app ELFs, so the workflow file is unchanged. Its comment "Does not run EL0 on the secondary" is now stale; fixing it needs a separate workflow-file push and is left for later.

## TCG timing (the honesty question)

- **QEMU TCG does not model ASID-tagged TLB retention across an ASID change.** On a TTBR0 write that changes the ASID, QEMU flushes that vCPU's soft TLB. So under TCG the existing `asid: ok` (ADR-013) and the ASID-switch part of t2t pass trivially. For the same reason the reissue probe keeps core 1 on one TTBR0 value from T1 through T2 (T1 "exits" without switching back to ASID 0). On hardware the stale entry would also survive an ASID switch, so the probe still models the real hazard, but it does not show ASID tagging itself.
- What the probe does show under TCG: the only thing between core 1's cached entry and its next access is core 0's rollover flush. With `tlbi vmalle1is` core 1 faults; with local `tlbi vmalle1` core 1 reads the retired page.
- **Observed on the Linux box, QEMU 10.0.13 TCG, `-cpu cortex-a76`, `-smp 2`, 2026-10-19:** the S4 positive build printed `smp: ok cpu1 s4 …` (13/13 probes ok on core 1, reissue fault) on **20 of 20** runs. The S4 leak build printed `smp: asidgen leak` and `smp: cpu1 probes missed failed=7 n=13` on **20 of 20** runs. The S3 `local-tlbi-leak-probe` still printed `smp: xtlb leak` on **20 of 20** runs on this QEMU (S3's 27/27 were measured on an earlier QEMU on the box). The full `CTOS_SMP2_SMOKE=1 ./scripts/qemu-smoke.sh` passed (default smoke, single-core leak builds, S1–S4 positive and all negatives).
- If a future QEMU stops reproducing a leak, the negative fails loudly instead of passing silently. The fallback is the same as in ADR-103: KVM on real arm64 (ADR-099 pilot path, sponsor approval needed).

## What this proves

- Two cores each have their own `TPIDR_EL1` → `PerCpu` block, own exception and thread stacks, and own copies of the EL0 context and expected-fault flags. The EL0 trampolines restore each core's own kernel TTBR0 through it.
- On QEMU TCG `-smp 2`, every EL0 isolation probe of the single-core boot also passes when run on core 1 (kernel-driven EL0, IRQ / FIQ from core 1's own timer via the lower-EL trampolines). Every existing leak build is still caught there, with its ok marker withheld.
- The ASID allocator hands out unique `(generation, asid)` pairs to two cores at once across a rollover. A rollover's broadcast flush removes core 1's stale entry for a reissued ASID; a local flush does not.

## What this does not prove

- Not SMP support or safety; not "EL0 isolated" on two cores. ADR-091 rev 4 stays one core.
- **The cores run probes one at a time.** Core 0 waits while core 1 runs EL0. The globals that are *not* banked are the result / "ready" flags core 0 publishes at boot and the probe bookkeeping in `syscall.rs`, `pan.rs`, `loader.rs`, `reach.rs`, `t2t.rs`, `xo.rs`, `teardown.rs`. So are the shared user tables (`user_ttbr0`, ASID-B), the shared EL0 probe page, and the scheduler. Running EL0 on both cores at once would race on these.
- The fatal / nested-exception stack is still one linker stack shared by both cores. Core 1's stacks have no guard pages. The `guard`, `wx`, `ro`, `sched`, standing-task and sample probes were not run on core 1.
- The allocator is not used by the default build or by the probes' fixed ASIDs 1 and 2. There is no scheduler, so `switch_to` is only exercised by the probe. The "reserved across rollover" path is not exercised by a probe (no task runs on core 1 during core 0's rollovers in the reissue probe).
- One reissue scenario, one direction (core 0 rolls over, core 1 faults). Nothing about real hardware, KVM or other QEMU versions.
- The SError-while-standing probe on core 1 parks (`el0: serror-park`), as on one core with this QEMU: `inject-nmi` is not available on stock QEMU. The taken-SError path was not shown on core 1.

## Proposed S5 (for sponsor go; not started)

**Done as [ADR-105](ADR-105-smp-s5-concurrent-el0-runq.md)** (sponsor go 2026-10-19 21:26 CEST).

Scheduler and concurrency: a run-queue lock and per-core current-task pointers (R6); EL0 tasks on **both** cores at the same time, each with an allocator ASID and its own user tables (no shared EL0 probe page); bank or retire the remaining global probe flags; a per-core fatal stack and guard pages for core 1; a probe that runs t2t / reach / xo on both cores at once with a rollover in between, plus a negative that drops the reserved-running-ASID rule. Then, **docs only and only with a written sponsor accept**, a proposal for the multi-core sentence (scope, the TCG limits above, and what KVM evidence it would need).

## Consequences

- Default kernel: the TPIDR indirection costs one extra load per lower-EL exception. The 71 banked statics double in size. Behaviour and the full default smoke (including the single-core leak builds) are unchanged.
- The opt-in CI job builds six ELFs (the positive, also run under `-smp 1`; three S2 negatives; the S3 leak; the S4 leak). It stays not required, within its 30-minute timeout.
