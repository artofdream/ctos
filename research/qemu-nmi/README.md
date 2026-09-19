# Research: local QEMU patch sketch for ADR-082 B1

**Status:** research only (2026-09-19). **Not** wired into Dockerfile, GHA, or `qemu-smoke.sh`.
**Not** proof of taken SError. Do not claim Verified from this directory alone.

## Why this exists

Sponsor chose ADR-081 **B1** (upstream / pinned QEMU). Upstream `virt` through
QEMU **master** (inspected 2026-09-19) still does **not** implement `TYPE_NMI`,
so QMP `inject-nmi` returns `machine does not provide NMIs`.

The 2020 qemu-devel series that would have raised `ARM_CPU_SERROR` from
`TYPE_NMI` never merged. FEAT_NMI in tree is an architectural **interrupt** NMI
(`ARM_CPU_NMI`), not async SError.

## What a future pin must prove

1. Build a pinned QEMU (commit SHA recorded) with a reviewed patch.
2. On agent-box + GHA: `inject-nmi` returns success **and** ctos serial shows
   `el0: serror` under EXPECT (not merely park).
3. Smoke then fail-closed requires `el0: serror`; park remains honesty for the
   failure path only if inject is optional (prefer fail-closed taken).
4. Separate ADR (or ADR-082 amendment) cites the pin SHA + green smoke.

## Draft patch

See `0001-hw-arm-virt-TYPE_NMI-raise-SError.patch`. It is a **sketch** against
QEMU 10.x-era APIs and may need retargeting to whatever `target/arm` interrupt
line exists for async SError on the chosen base commit (current trees expose
`CPU_INTERRUPT_VSERR` / FEAT_NMI paths; a correct patch must raise the guest
**async SError** taken by ctos `handle_serror_lower_el`, not FIQ/BRK/FEAT_NMI).

Do **not** apply this in CI until sponsor review + guest proof.

## Reproduce the blocker without a patch

```sh
./scripts/qemu-nmi-probe.py
```
