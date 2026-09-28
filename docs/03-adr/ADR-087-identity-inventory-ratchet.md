# ADR-087 — Identity inventory ratchet + RAM-side identity leftovers tear (M1)

- Status: Accepted (serial / tests / smoke). **M1** of the [ADR-086](ADR-086-el0-isolated-checklist-audit.md) plan. It unmaps identity leftovers I2 / I4 / I5 and commits a fail-closed TTBR0 identity inventory. Allowed set = {I1 MMIO block, I3 `_start` stub page}. A planted identity page must be caught in-boot, and a dedicated `inv-leak-probe` kernel must fail the check. The umbrella “EL0 isolated” stays **Planned / non-claim**.
- Date: 2026-09-26
- Base: stacked on PR [#138](https://github.com/artofdream/ctos/pull/138) (`3f4de00`). #136 / #137 / #138 are unmerged.
- Sponsor decisions (DSO relay ~14:35 CEST) arrived while this ran: D2 keeps I3 as the one documented identity exception, and D3 puts MMIO in scope (M2). This ADR does not act on them beyond keeping I1 / I3 allowed for now.

## Context

The ADR-086 scratch walker found five identity (VA = PA) ranges left in kernel TTBR0 after the ADR-018…049 tears: I1 MMIO 1 GiB Device block, I2 low RAM `0x4000_0000–0x4008_0000`, I3 boot-stub page `0x4008_0000`, I4 RO+X linker padding `[.ident_tear end, 0x4020_1000)`, and I5 the pre-heap frame at `__kernel_end`. User TTBR0 still held I3 + I4. Those tears were section-driven, so nothing ratcheted “what is still identity-mapped”.

## Decision

1. **Tear I2 / I4 / I5** (`paging::tear_identity_leftovers`, right after `tear_identity_ram`) from kernel **and** user TTBR0. ASID-B shares the kernel RAM L2. The ranges are `paging::leftover_ranges()`: `[VIRT_RAM_BASE, KERNEL_TEXT)`, `[boot_stub_end, data_start)`, and `[data_tear_end, heap_pa)`. `pages=` counts only pages that were still mapped. High twins stay. One boot-time `TLBI VMALLE1`. `_start` is never touched: the function refuses any range overlapping the stub page.
2. **Fault probes.** An EL1 `LDR` of a torn leftover VA must take a current-EL translation DABORT: `ident: low-fault` / `ident: tail-fault` / `ident: kend-fault`. `is_torn_identity_va` learns the three ranges.
3. **Inventory ratchet** (`paging::identity_inventory`). It walks kernel, user and ASID-B TTBR0 (L1→L3), coalesces identity leaves, and prints each as `ident: inv-range` (allowed) or `ident: inv-leak` (not allowed, or EL0-accessible), then `ident: inv k=<n> u=<n> a=<n> leaks=<n>`. `ident: ok` now requires `leaks=0`.
4. **In-boot negative probe** (`inventory_negative_probe`). It plants an identity page at `0x4000_0000` in kernel TTBR0 and requires the walk to flag it, then does the same in user TTBR0. It then removes the plant and requires a clean walk: `ident: inv-neg k caught` / `ident: inv-neg u caught` / `ident: inv-neg clean` → `ident: inv-ok allow=mmio,stub`.
5. **Smoke-level negative probe.** Cargo feature `inv-leak-probe` leaves the plant in place. `scripts/qemu-smoke.sh` builds and boots it and **fails** unless the serial shows `ident: inv-leak k lo=0x40000000` and **no** `ident: inv-ok` / `ident: ok`. This is CI-gated without a workflow edit.
6. **Smoke** requires the markers in 1–4 plus `leaks=0` and still requires `ident: start-stay`. It rejects `ident: inv-leak`, `ident: inv missed`, `ident: inv-neg missed`, and `ident: left missed`. `#[test_case]`s: `identity_leftovers_torn_stub_stays`, `identity_inventory_allowlist_only_catches_plant`.
7. **Doc drift fix:** ADR-055 §A now says `el0: no kernel read`, which is what code and smoke use. The earlier text said `el0: no data`. ADR-086 also misstated the rustc on the box smoke: the repo pin is `1.100.0-nightly 0fc141305`, not `5ceaf6608`. This is corrected in place.

## Evidence (this session)

Agent box, 2026-09-26 14:35–14:40 CEST, `scripts/qemu-smoke.sh` → `qemu-smoke: ok` (QEMU 10.0.13 Debian, `virt -cpu cortex-a76` TCG, rustc 1.100.0-nightly `0fc141305`). Hello boot:

```
ident: low lo=0x40000000 hi=0x40080000 pages=128 user=0
ident: tail lo=0x400f3000 hi=0x40201000 pages=270 user=270
ident: kend lo=0x4025d000 hi=0x4025e000 pages=1 user=0
ident: start-stay lo=0x40080000 hi=0x40081000
ident: low-fault va=0x40000000
ident: tail-fault va=0x40200000
ident: kend-fault va=0x4025d000
ident: inv-range k lo=0x0 hi=0x40000000 dev rw nx
ident: inv-range k lo=0x40080000 hi=0x40081000 mem ro x
ident: inv-range u lo=0x40080000 hi=0x40081000 mem ro x
ident: inv-range a lo=0x0 hi=0x40000000 dev rw nx
ident: inv-range a lo=0x40080000 hi=0x40081000 mem ro x
ident: inv k=2 u=1 a=2 leaks=0
ident: inv-neg k caught va=0x40000000
ident: inv-neg u caught va=0x40000000
ident: inv-neg clean
ident: inv-ok allow=mmio,stub
ident: ok
```

`cargo test`: `identity_inventory_allowlist_only_catches_plant [ok]`, `identity_leftovers_torn_stub_stays [ok]`. `inv-leak-probe` kernel:

```
ident: inv-leak-probe planted va=0x40000000
ident: inv-leak k lo=0x40000000 hi=0x40001000 mem rw nx
ident: inv-leak a lo=0x40000000 hi=0x40001000 mem rw nx
ident: inv k=3 u=1 a=3 leaks=2
ident: inv missed
qemu-smoke: inv-leak-probe caught (fail-closed ok)
```

(ASID-B sees the plant because it shares the kernel RAM L2.)

Before → after (kernel TTBR0 identity ranges): **5 → 2** (I1 MMIO, I3 stub). User TTBR0: **2 → 1** (I3).

`b2-serror` profile under stock TCG (`CTOS_B2_ACCEL=tcg`): boots through `ident: low/tail/kend` to `el0: serror-arm` → `el0: serror-park`, the expected result without patch 0002. **Not** re-run on KVM: no metal was used in this phase.

## Surprise: the pointer rewrite also rewrote the allowlist

The first run flagged the stub page as a leak. `INV_ALLOW` was promoted into `.rodata`, and the ADR-020/025 identity-pointer rewrite (`ident: ro-reloc n=1418`) treated the words `0x4008_0000` / `0x4008_1000` as identity text pointers and rewrote them to TTBR1 aliases. The check now compares `identity_pa()` of the loaded bounds. Lesson: any `.rodata` constant that looks like a kernel identity address is silently relocated. Security-relevant constants must be compared physically or encoded as immediates.

## Honesty

Say: “RAM-side identity leftovers (low RAM, image padding tail, pre-heap frame) are unmapped from kernel and user TTBR0. A fail-closed walk proves only the MMIO block and the `_start` stub page remain identity-mapped, and a planted identity page is caught.” Do **not** say:

- identity mappings are fully torn down (MMIO + stub remain)
- `_start` was yanked / the kernel moved
- “EL0 isolated” / “secure OS”
- the b2 profile was re-verified on KVM after this change

## Consequences

- Allowed set is {I1, I3}. M2 ([ADR-086](ADR-086-el0-isolated-checklist-audit.md) plan, D3 accepted) moves MMIO high and shrinks it to {I3}. M3 records I3 as the one documented exception (D2).
- Ledger, roadmap P-SEC-3l, threat model **v1.62**, [el0.md](../framework/el0.md), SUMMARY, second brain updated. Do not self-merge.
