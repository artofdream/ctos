# ADR-108 — Taken-SError QEMU spike: what triggers it on TCG (stock and pinned)

- Status: Accepted (spike / research). Sponsor go relayed by DSO on 2026-10-23 at 19:25 CEST (stop-all lifted for ctos only). One finding, one PR. No change to the kernel, `_start`, the default smoke or the ADR-091 claim.
- Date: 2026-10-23
- Base: main `c420fba` (includes #160, #161, #162).
- Machine: amd64 workstation (bare-metal Ubuntu, TCG only; no KVM for an aarch64 guest). No cloud, no CloudAgent. Builds ran in an `ubuntu:24.04` container on that host because the host lacks QEMU build deps and root.
- Number: ADR-107 is taken by open PR #163, so this is ADR-108.

## Question

Find a QEMU setup that can trigger and observe a **taken** SError, as groundwork for the "EL0 isolated" claim, and say exactly what is and isn't shown today.

## What is shown today (before this spike)

| Evidence | Where | Shows | Does not show |
| --- | --- | --- | --- |
| B1 `QEMU NMI pin smoke (taken SError)` CI job | [ADR-083](ADR-083-b1-qemu-nmi-pin.md), [ADR-090](ADR-090-serror-evidence-class.md) | Pinned QEMU v10.0.0 + patch 0001; QMP `inject-nmi` → bare `el0: serror` while the full ctos image stands at EL0 on **cpu0**, `-smp 1`. Fail-closed (`CTOS_REQUIRE_TAKEN_SERROR=1`). | Stock QEMU; cpu1 or any secondary; an SError source other than the repo's own QEMU patch; hardware RAS. |
| B2-P (one-off KVM, Graviton3) | [ADR-085](ADR-085-b2p-graviton-kvm-serror.md) | Historical run 4 at `159b178`, reduced profile. | Current boot path; repeatability; default image. |
| Default smoke on stock QEMU | ADR-053/081 | `el0: serror-park`: stock `virt` refuses `inject-nmi` ("machine does not provide NMIs"). | Any taken SError. |
| SMP S4/S5 | [ADR-104](ADR-104-smp-s4-percpu-asid-generation-el0-cpu1.md), [ADR-105](ADR-105-smp-s5-concurrent-el0-runq.md) | cpu1 runs EL0 probes. | No SError probe is taken on cpu1; it parks on stock QEMU. |

## Options tried (exact versions, this session)

QEMUs:
- **stock A:** distro `qemu-system-aarch64` 10.2.1 (`1:10.2.1+ds-1ubuntu3.2`) on the host.
- **stock B:** Ubuntu 24.04 `qemu-system-arm` 8.2.2 (`1:8.2.2+ds-0ubuntu1.18`), the same version as the GHA runners.
- **upstream master:** `b6c4b7e` (2026-10-23, `VERSION` 11.1.50), built from source.
- **pinned B1:** v10.0.0 `7c949c53` + `research/qemu-nmi/0001-…patch`, via `./scripts/build-qemu-nmi.sh`.

| Route | Result |
| --- | --- |
| QMP `inject-nmi` / HMP `nmi` on stock A, stock B, master | **Refused**: `Error: machine does not provide NMIs`. Master `hw/arm/virt.c` still has no `TYPE_NMI`. |
| Same on **pinned B1** | **Works**, and targets a chosen vCPU: HMP `nmi` with QMP `human-monitor-command` `cpu-index=1` raises the SError on **cpu1** only. |
| gdbstub (`-gdb`, `monitor nmi`) | It is the **same** HMP `nmi` (forwarded over qRcmd): refused on stock, taken on pinned. gdb has no SError injection of its own, and writing registers would be a fake. Rejected as a separate route. |
| Bad device access (EL0 load from unassigned PA 4 GiB) | **Synchronous** external abort: vector 0x400, EC 0x24, DFSC 0x10. It is **not** an SError. Upstream `target/arm/tcg/translate-a64.c` says: "QEMU does not have a source of physical SErrors, so we are only concerned with virtual SErrors"; `arm_cpu_do_transaction_failed` raises a sync abort. Rejected (negative row kept). |
| **Virtual SError** (`HCR_EL2.VSE` + `AMO`, guest boots at EL2 with `-M virt,virtualization=on`) | **Works on every QEMU tried, including stock**: taken at EL1 from EL0, vector `VBAR_EL1+0x580`, ESR_EL1 `0xbe000000` (EC 0x2f), on cpu0 and on cpu1 (PSCI `CPU_ON`, `-smp 2`). Architectural, no QEMU patch. |
| RAS extension registers (`ERX*`) / FEAT_RAS error injection | Not tried in code: TCG has no physical SError source (see above), so there is nothing to drive. FEAT_NMI is an interrupt and does not count as an SError ([ADR-082](ADR-082-b1-qemu-type-nmi.md)). |
| KVM | Not available: the host is amd64. |

## Finding (one)

**On TCG there are exactly two honest ways to get a taken SError: (a) the pinned B1 patch, which is physical and host-injected and can now target cpu1; and (b) the architectural virtual SError via `HCR_EL2.VSE`, which works on stock QEMU 8.2.2, 10.2.1 and master, on cpu0 and cpu1, but only if the guest owns EL2.** Bad-PA accesses and the gdbstub add nothing new.

Evidence: [`research/serror-spike/`](../../research/serror-spike/README.md). The matrix has 13 fail-closed rows per QEMU, 4 QEMUs, 52/52 ok. Log: `research/serror-spike/2026-10-23-matrix-log.txt`.

Negatives (all pass, on every QEMU):
- VSE without `AMO` is not routed;
- `AMO` without VSE gives no SError;
- VSE with EL0 `PSTATE.A` set is masked;
- with the pin, an `nmi` to cpu0 while cpu1 stands at EL0 does not reach cpu1, and an `nmi` with EL0 A-masked is not taken;
- with stock QEMU, every inject row must show the refusal and no SError.

## Decision

1. Record the finding. Add the stand-alone spike image (`research/serror-spike/vse`), `serror-matrix.py` (fail-closed, exit 1 on any mismatch) and `gdb-nmi.sh`. **No CI job is added**, and the required checks are unchanged.
2. **No claim changes.** The ADR-090 evidence class and the ADR-091 rev 4 sentence are untouched. This spike uses a **stand-alone image, not the ctos kernel**.
3. Possible next steps, each needing a sponsor go:
   - **N1 (pin, cpu1):** extend B1 to `-smp 2` with `cpu-index=1` while ctos cpu1 stands at EL0 (S4/S5 path). Needs a cpu1 `el0: serror` arm/handler in ctos.
   - **N2 (stock, virtual):** a small EL2 shim, loaded with `-device loader` ahead of the kernel and **not** touching `_start`, that sets `HCR_EL2.{RW,AMO}`, ERETs to ctos at EL1, and sets VSE on an HVC from ctos's `serror-arm` point. This would give a taken SError on stock QEMU with the full image. It is a *virtual* SError (EL2-injected), and its honesty wording must say so.

## Honesty

Say: "On QEMU TCG, a taken lower-EL SError is produced only by (a) the repo's pinned TYPE_NMI patch, which can target cpu1 too, or (b) an architectural virtual SError from EL2 (`HCR_EL2.VSE`), which works on stock QEMU; shown on a stand-alone test image with negatives."

Do **not** say:
- that ctos takes SErrors on cpu1;
- that stock QEMU has a physical SError source;
- that bad device accesses produce SErrors on QEMU;
- that this changes the "EL0 isolated" claim.

## Consequences

- New: this ADR, `research/serror-spike/` (README, vse image source, matrix script, gdb script, log), a brief, a ledger row, SUMMARY and moc entries.
- No kernel, workflow, Dockerfile or smoke change. The default single-core build is unchanged.
