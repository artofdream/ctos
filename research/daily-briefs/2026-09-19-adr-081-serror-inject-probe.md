# ADR-081 research brief — taken lower-EL SError inject (agent-box)

- Date: 2026-09-19 CEST
- Host: agent-box (`grok-bot-vm-401670533`)
- QEMU: `qemu-system-aarch64` **10.0.13** (Debian `1:10.0.13+ds-0+deb13u1`)
- Accel: TCG only (`query-kvm` → `present=false`; `-accel kvm` → `invalid accelerator kvm`)
- Default smoke CPU: `-cpu cortex-a76` (ADR-079)

## Question

Is there an honest host path to deliver ARM **async SError** to standing EL0 on the accepted smoke machine (QEMU `virt` TCG `-kernel`), so smoke can require `el0: serror` under EXPECT?

## Matrix (machine paused with `-S`; QMP only — inject availability does not need the ctos kernel)

| Config | QMP `inject-nmi` | HMP `nmi` |
| --- | --- | --- |
| `virt` + `cortex-a76` (default) | `machine does not provide NMIs` | same |
| `virt` + `cortex-a57` (historical) | same | same |
| `virt,gic-version=3` + `cortex-a76` | same | — |
| `virt,gic-version=3` + `max` | same | — |
| `virt,virtualization=on` + `cortex-a76` | same | — |
| `virt,gic-version=3,virtualization=on` + `cortex-a76` | same | — |
| `virt,gic-version=3,secure=on` + `cortex-a76` | same | — |

`query-commands` lists `inject-nmi`, but the virt machine does **not** implement `TYPE_NMI`. Other `*-inject-*` commands are **CXL-only** — not an ARM SError path.

`-cpu max,nmi=on` fails at QEMU startup: `Property 'max-arm-cpu.nmi' not found` (not a silent FEAT_NMI knob on this build).

## Not SError (explicitly rejected)

- **FEAT_NMI / FEAT_GICv3_NMI** — architectural *interrupt* NMI story (GICv3 + `aa64_nmi`); not async SError / `ARM_CPU_SERROR`. Do not invent FIQ/BRK/FEAT_NMI as taken SError.
- **Guest-only RAS / poisoned load** — still rejected for Verified (ADR-044 H2).
- **KVM `serror_pending` / plugin inject** — out of scope; this host has no KVM accel; CI smoke is TCG `virt` `-kernel`.

## Historical note

A 2020 qemu-devel series (`hw/arm/virt: Simulate NMI Injection`) would have wired `TYPE_NMI` → raise `ARM_CPU_SERROR`. It did **not** land in upstream QEMU 10.0.13 virt. Current `inject-nmi` still errors with `machine does not provide NMIs` (see also qemu-devel 2026-08 `nmi_inject` wording).

## Unlock (sponsor decision)

Taken SError Verified needs **one** of:

1. Upstream (or sponsor-pinned) QEMU where `virt` implements `TYPE_NMI` and `inject-nmi` raises **async SError** (`ARM_CPU_SERROR`) on the accepted CPU/GIC — then re-probe + ADR + `el0: serror` under EXPECT; **or**
2. A sponsor-approved smoke-machine change (documented ADR) that provides an honest async-SError inject CI can run (not inventing FIQ/BRK; not claiming FEAT_NMI is SError); **or**
3. Explicit permanent non-goal if sponsor declines further machine work.

Until then: park Verified; A-clear dormant prep; do **not** require `el0: serror`.
