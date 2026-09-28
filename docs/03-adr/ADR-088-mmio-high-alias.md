# ADR-088 — Device MMIO through a TTBR1 high alias; identity MMIO block torn (M2)

- Status: Accepted (serial / tests / smoke). **M2** of the [ADR-086](ADR-086-el0-isolated-checklist-audit.md) plan. Sponsor **D3** (DSO relay 2026-09-26 ~14:35 CEST): device MMIO is in scope for identity teardown. GIC / PL011 / virtio-mmio now use TTBR1 Device blocks. The identity MMIO 1 GiB L1 block (**I1**) is cleared. The identity inventory's allowed set shrinks to **{I3 `_start` stub page}**. The umbrella “EL0 isolated” stays **Planned / non-claim**.
- Date: 2026-09-26
- Base: stacked on PR [#139](https://github.com/artofdream/ctos/pull/139) (ADR-087, M1).

## Decision

1. **TTBR1 Device alias.** `paging::init` adds three 2 MiB Device-nGnRnE blocks (EL1 RW, PXN+UXN, no EL0 access) to `L2_HIGH` at `TTBR1_BASE + pa`: `0x0800_0000` (GICD + GICC), `0x0900_0000` (PL011), `0x0a00_0000` (virtio-mmio transports). That is 6 MiB instead of the 1 GiB identity hole. The block list is built from immediates, because the ADR-087 finding showed that `.rodata` tables of kernel-looking words get rewritten.
2. **Driver switch.** `paging::mmio_va(pa)` returns the identity PA until `enable_mmio_high()` and the high alias afterwards. `uart`, `gic` and `virtio` register accessors all go through it. `enable_mmio_high()` runs first thing in `kernel_main_high` (after the high jump, before any tear) and prints `ident: mmio-high gic=… uart=… virtio=…`. The flag is a plain atomic load, so the UART is safe pre-MMU, in fatal paths, and while `TABLES` is held.
3. **Tear I1.** `tear_identity_mmio()` runs after the ADR-087 leftovers tear. It clears kernel `L1[0]` and the ASID-B copy; user TTBR0 never had it. One boot-time `TLBI VMALLE1`, then it verifies the GIC / PL011 / virtio identity VAs are unmapped: `ident: mmio lo=0x0 hi=0x40000000 l1=1`. `prepare_asid_b_tables` no longer requires an identity MMIO slot.
4. **Fault probe.** An EL1 `LDR` of the torn identity `UARTFR` (`0x0900_0018`) must take a translation DABORT before any device access: `ident: mmio-fault va=0x9000018`.
5. **Allowlist shrink.** `INV_ALLOW = {[0x4008_0000, 0x4008_1000)}`. The smoke requires `ident: inv k=1 u=1 a=1 leaks=0` and `ident: inv-ok allow=stub`, and rejects any `ident: inv-range k|a lo=0x0`. W^X keeps “MMIO is XN” via `paging::mmio_xn`, which checks whichever alias is live.
6. `#[test_case]` `identity_mmio_torn_high_alias_xn`.

## Evidence (this session)

Agent box 2026-09-26 ~14:43–14:44 CEST `scripts/qemu-smoke.sh` → `qemu-smoke: ok` (QEMU 10.0.13, cortex-a76 TCG, rustc `0fc141305`):

```
ident: jump
ident: mmio-high gic=0xffffff8008000000 uart=0xffffff8009000000 virtio=0xffffff800a000000
ident: low lo=0x40000000 hi=0x40080000 pages=128 user=0
ident: tail lo=0x400f3000 hi=0x40201000 pages=270 user=270
ident: kend lo=0x4025d000 hi=0x4025e000 pages=1 user=0
ident: mmio lo=0x0 hi=0x40000000 l1=1
Hello World!
ident: start-stay lo=0x40080000 hi=0x40081000
ident: mmio-fault va=0x9000018
ident: inv-range k lo=0x40080000 hi=0x40081000 mem ro x
ident: inv-range u lo=0x40080000 hi=0x40081000 mem ro x
ident: inv-range a lo=0x40080000 hi=0x40081000 mem ro x
ident: inv k=1 u=1 a=1 leaks=0
ident: inv-neg k caught va=0x40000000
ident: inv-neg u caught va=0x40000000
ident: inv-neg clean
ident: inv-ok allow=stub
ident: ok
```

All UART output after `ident: mmio-high` went through the TTBR1 alias, as did the virtio-blk / virtio-net / FAT / GIC timer and IRQ / FIQ probes. `wx: ok` and `cargo test` `identity_mmio_torn_high_alias_xn [ok]`. The `inv-leak-probe` kernel is still caught.

Identity ranges: kernel TTBR0 **2 → 1**, ASID-B **2 → 1**, user **1** (unchanged). Only the `_start` stub page remains.

The `b2-serror` profile under stock TCG boots through `ident: mmio-high` / `ident: mmio` to `el0: serror-arm` → park, the expected result without patch 0002. It was **not** re-run on KVM. The ADR-085 B2-P evidence was taken at `159b178`, before M1/M2 changed the boot path (tears + MMIO alias). A KVM re-run would cost about $0.2 (≈ 45 min of `c7g.metal` Spot). Not done in this phase.

## Honesty

Say: “device MMIO is reached only through EL1-only TTBR1 Device blocks; the identity MMIO block is unmapped; the only identity mapping left in any TTBR0 is the `_start` stub page, and a fail-closed walk proves it.” Do **not** say:

- identity teardown is “complete”. I3 remains, as the documented exception (D2 / M3).
- `_start` was yanked / the kernel moved
- “EL0 isolated” / “secure OS”
- the b2 profile was re-verified on KVM after M1/M2

## Consequences

- ADR-055 §B row 2 (as reworded by M3 per D2) has its evidence: identity = {stub} only. M3 records the exception; the umbrella still needs M4 (D1 evidence), M5 (draft accept) + sponsor accept, and M6.
- Ledger, roadmap P-SEC-3l, threat model **v1.63**, [el0.md](../framework/el0.md), SUMMARY, second brain. Do not self-merge.
