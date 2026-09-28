# ADR-098 — PAN held while the kernel handles EL0 exceptions (correction of a live-claim defect)

- Status: Accepted (implementation + evidence). Fixes a defect in the accepted [ADR-091](ADR-091-el0-isolated-accept-draft.md) claim. It does **not** change the ADR-091 sentence or scope.
- Date: 2026-09-29
- Decision path: DSO relay (2026-09-28 23:54 CEST). The defect was found while working on ADR-097 (PR #150, execute-only app text, which waits for a sponsor rev 4 accept). It is split out here so the fix can merge without waiting for that accept.
- Base: `main` at `9a1c2bb` (M6 flip, PR #148). Not stacked on #149 or #150.
- `_start` (`0x4008_0000`) untouched.

## Context: the defect

ADR-091's accepted sentence says “PAN is enabled and EL1 faults on EL0 memory”. The evidence is ADR-080's boot probe (`pan: enabled` / `pan: el1-fault`). That probe runs once, early in boot, **before any EL0 trip**. Two things in the exception path undid PAN once EL0 had run:

1. **`SCTLR_EL1.SPAN` was 1** (the QEMU reset value; the kernel never cleared it). With SPAN=1, taking an exception to EL1 leaves PSTATE.PAN unchanged. EL0 was entered by ERET with an SPSR whose PAN bit (22) is clear, so every exception from EL0 (SVC, IRQ, FIQ, abort) started its EL1 handler with **PAN clear**. PAN came back only if the handler called a copy helper (`pan::with_user_access` restores it after the copy).
2. **`return_from_el0` resumed the kernel with SPSR `0x3C4`** (PAN clear). After any EL0 trip that ended by returning to the kernel continuation, the kernel kept running with **PAN clear** until the next `with_user_access` set it again.

Measured before the fix (reproduced by the `pan-keep-leak-probe` build below): in the loaded app's `SYS_YIELD` handler `MRS PAN` reads 0 and `SCTLR_EL1.SPAN` is 1; after the app exits, `MRS PAN` reads 0 and an EL1 load of an EL0-readable page does **not** fault. The boot probe still prints `pan: enabled` and `pan: el1-fault` in that same kernel.

**Consequence for the claim:** from the ADR-080 enable (2026-09-19), and so also from the M6 accept (2026-09-28 16:41 CEST) until this fix, the clause “PAN is enabled” held at boot, but **not** while the kernel handled EL0 exceptions, and not after EL0 trips until a copy helper ran. The boot probe missed this because it ran before any EL0 trip. EL0 itself could not exploit it directly (EL0 still could not reach kernel memory, and the syscall pointer checks of ADR-094 do not depend on PAN), but PAN's purpose is to catch *kernel* bugs that dereference user pointers, and that net was down exactly when the kernel handles EL0 requests.

## Decision

1. After a successful ADR-080 enable, clear `SCTLR_EL1.SPAN` (bit 23). Every exception taken to EL1 then sets PSTATE.PAN.
2. `return_from_el0` sets `SPSR.PAN` (bit 22) when PAN is enabled, so the kernel resumes with PAN set after an EL0 trip.
3. Copy helpers are unchanged: `with_user_access` still clears PAN around an intentional copy and restores it.

## Probes (fail-closed in `scripts/qemu-smoke.sh`)

- **Live, during an EL0 exception:** called from the ADR-092 live walk in the `SYS_YIELD` handler of the loaded `hello-libctos` app: `pan: live-syscall pstate-pan=1 sctlr-span=0`. An EL1 load cannot be the test here: once PAN is really set it takes a current-EL permission fault inside an exception handler, which the kernel treats as fatal by design. So the handler reads `MRS PAN`.
- **After EL0 trips:** right after the loaded app exits: `pan: after-el0 trips=<n> mrs-pan=1 el1-load va=0x80003000 fault=yes spsr-pan=1` (n ≥ 1 returns from EL0 so far; an EL0-readable page is mapped at the transient `crt-stack-pan` slot VA, loaded with the ADR-080 catcher armed silently, then unmapped).
- **Summary:** `pan: ok held boot,live-syscall,after-el0`, printed only if both checks passed.
- **Calibration of `MRS PAN`:** it reads 1 in the after-EL0 check, where the real EL1 load faults with SPSR.PAN set; it reads 0 in the negative build, where the load does not fault. (ADR-080 noted that `MRS PAN` “may read 0” under QEMU 10.0.13 TCG; in this build, whenever a fault proved PAN set, `MRS PAN` read 1.)
- `#[test_case]` `pan_span_cleared_after_enable` (SPAN = 0 and `MRS PAN` = 1 in kernel context under `cargo test`).
- The smoke requires the three lines and rejects `pan: lost`, `pan: bad after-el0`, `pan: held missed`, `pan: held probe missed`. Existing `pan: id=` / `pan: present` / `pan: enabled` / `pan: el1-fault` checks are unchanged.

## Negative tests

- **Build:** the existing `el0-leak-probe` smoke build also enables `pan-keep-leak-probe`, which undoes both parts of the fix (SPAN stays 1; `return_from_el0` resumes with SPSR.PAN clear). The smoke requires, from that kernel: `pan: enabled` and `pan: el1-fault` (the boot probe still passes — the blind spot), `pan: lost live-syscall pstate-pan=0 sctlr-span=1`, `pan: lost after-el0 trips=<n> mrs-pan=0 el1-load va=0x80003000 fault=no spsr-pan=0`, `pan: held missed live-taken=true live-ok=false after-el0=false`; and fails if `pan: ok held`, `pan: live-syscall pstate-pan=1` or `pan: after-el0 ` appears.
- **Tampered logs** (the ADR-098 block against edited copies of a good box serial; each must fail): `pan: ok held` deleted; live line with `pstate-pan=0 sctlr-span=1`; after-EL0 line with `fault=no spsr-pan=0`; `trips=0`; live line deleted; a `pan: lost` line appended; and the real negative-build serial. All seven failed with a named missing / forbidden line; the unedited log passed. The leak-build section passes on the negative-build serial and fails on the good serial.

## ADR-091 clause check after this fix

Every clause of the accepted revision-3 sentence is literally true again on the default smoke machine, and now continuously for the PAN clause: PAN is enabled at boot, in EL0 exception handlers, and after EL0 trips; EL1 faults on EL0-readable memory (boot probe + after-EL0 check). The sentence is **not** edited. A dated correction note is added to ADR-091's status list (the same place as its 2026-09-28 limits update), and a correction row to the honesty ledger.

## Limits

- One core, QEMU TCG default smoke machine; not real hardware (no KVM re-run of this change), not side-channel, not a certification.
- In handlers the check is `MRS PAN` (calibrated), not an EL1 load; the live check samples one exception type (SVC `SYS_YIELD`). IRQ/FIQ/abort entries get the same SPAN=0 behaviour architecturally but are not sampled separately.
- The after-EL0 check samples one point (after the loaded app exits).
- Kernel paths that deliberately touch EL0 pages must keep using `with_user_access`; with PAN now held, one that doesn't would fault (and in a handler, park). The full smoke (all EL0 apps, FAT / net / TCP samples, A9 cross-update, `cargo test`) is green with the fix.
