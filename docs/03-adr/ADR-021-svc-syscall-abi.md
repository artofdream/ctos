# ADR-021 — Minimal EL0 SVC syscall ABI

- Status: Accepted (ABI mile Verified when the serial / tests pass; Track A / app hosting still Planned)
- Date: 2026-09-11

## Context

Track A ([issue #31](https://github.com/artofdream/ctos/issues/31)) wants freestanding apps to call the kernel. [Issue #32](https://github.com/artofdream/ctos/issues/32) is the first mile: a **documented, probed** SVC contract.

[ADR-013](ADR-013-el0-isolation-direction.md) already takes lower-EL AArch64 `SVC` for probe immediates `#0` (first-mile return), `#1` (standing announce), and `#2` (standing restore). Those numbers stay reserved. This ADR adds a tiny public set so a later CRT / `libctos` (A2) has numbers to call. It does **not** load ELF, host apps, or claim Linux/POSIX.

Options:

1. **Linux AArch64 (`svc #0`, `x8` = number).** Collides with the first-mile `SVC #0` probe and invites a Linux-ABI claim.
2. **SVC immediate is the number, public set starts at 16.** Leaves 0–2 for ADR-013 probes. Arguments in `x0`–`x2`, return in `x0`.
3. **Wait for a loader.** Would block Track A on A3.

## Decision

1. **Calling convention.** AArch64 `SVC #<n>` where `n` is the syscall number. Arguments in `x0`, `x1`, `x2`. Return value in `x0`. DAIF stays masked on the existing EL0 path. Not Linux. Not POSIX.
2. **Reserved 0–2.** Probe-only ([ADR-013](ADR-013-el0-isolation-direction.md)). Not part of the public ABI. A later CRT must not issue them.
3. **Public numbers (this tree).**

   | Number | Name | Args | Effect | Return |
   | --- | --- | --- | --- | --- |
   | 16 | `exit` | `x0` = status (recorded; not a process model) | End the EL0 trip; `ERET` to the EL1 caller | (does not return to EL0) |
   | 17 | `uart_write` | `x0` = user pointer, `x1` = length | Copy up to 64 bytes from a user-mapped **and** kernel-mapped range; write to PL011 (`\n` → `\r\n`, same as the kernel console) | bytes written, or `0` if rejected |
   | 18 | `yield` | none | Record the call; `ERET` back to EL0 (next insn) | `0` |

4. **`yield` is dispatch-only.** It does **not** call `sched::yield_now` ([ADR-010](ADR-010-cooperative-rr-el1.md): that switch is EL1-only and must not run from an exception frame). A later ADR may connect the two.
5. **`uart_write` is fail-closed.** Length `0` or `> 64`, overflow, a non-canonical or TTBR1 alias, or a range that is not user-mapped **or** not kernel-mapped returns `0` and writes nothing. Kernel `.data` is rejected. Table walks mask to 39 bits — the pointer itself must already be a TTBR0 identity VA, or `copy_user` would load a tagged address. This is not a VFS.
6. **Unknown immediates** stay unhandled (park). Do not silently succeed.
7. **Fail-closed probe.** Hello serial `svc: yield` + user buffer `svc: user-hi` + `svc: uart` + `svc: exit` + `svc: ok`. `scripts/qemu-smoke.sh` greps those and rejects `svc: probe missed`. `#[test_case]` covers the success trip, a kernel-`.data` reject, and a TTBR1-alias reject. Existing `el0:` markers stay.
8. **Honesty.** Say “EL0 issued the documented SVC ABI and the kernel performed the documented effect” only when the serial / tests pass. Do **not** say: app hosting is done, Linux ABI, POSIX, userspace, “EL0 isolated,” or “secure OS.” Standing-as-normal is [ADR-024](ADR-024-standing-el0-normal.md). Track A A5–A9 stay Planned.
9. **NFR-10 text** is revised in place (ID unchanged) to name this ABI mile. Do not mint FR-16+ or NFR-15+.

## Consequences

- Code: `src/syscall.rs`. Dispatch is hooked from `handle_sync_lower_el` after the ADR-013 probe immediates.
- Docs: [syscall.md](../framework/syscall.md), [el0.md](../framework/el0.md). Roadmap cites #32 / Track A #31.
- A2 (`libctos`) wraps these three numbers ([ADR-022](ADR-022-libctos-crt.md)). A3 (loader) is not that PR.
