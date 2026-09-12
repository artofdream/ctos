# EL0 isolation (P-SEC-3 / ADR-013)

**Isolation: Planned.** A first mile, a user-TTBR0 read mile, an ASID TLB mile, a standing EL0 context, a TTBR1 private-page first cut, an EL1 high-VA fetch mile, an identity-tear first cut, an identity `.text` range tear, a live identity `.text` tear after a high-VA vtable rewrite, a **documented SVC ABI mile**, and a **`libctos` CRT mile** exist. Those miles are not app hosting. Track A stays incomplete after A2. Do not claim userspace, app hosting, or “EL0 isolated.” Standing enter/leave as a guest sample: [apps-today.md](apps-today.md). CRT: [building-or-porting.md](building-or-porting.md).

Direction: [ADR-013](../03-adr/ADR-013-el0-isolation-direction.md). TTBR1 first cut: [ADR-016](../03-adr/ADR-016-ttbr1-private-page.md). EL1 fetch mile: [ADR-017](../03-adr/ADR-017-ttbr1-high-el1-exec.md). Identity-tear first cut: [ADR-018](../03-adr/ADR-018-identity-teardown-first-cut.md). Identity `.text` range tear: [ADR-019](../03-adr/ADR-019-identity-text-range-tear.md). Live `.text` tear: [ADR-020](../03-adr/ADR-020-identity-fnptr-reloc.md). SVC ABI: [ADR-021](../03-adr/ADR-021-svc-syscall-abi.md) / [syscall.md](syscall.md). CRT / `libctos`: [ADR-022](../03-adr/ADR-022-libctos-crt.md). Threat model: [security.md](security.md). Code: `src/el0.rs`, `src/syscall.rs`, `src/libctos.rs`, `libctos/`, `src/asid.rs`, `src/ttbr1.rs`, `src/teardown.rs`.

## What exists today

- Kernel runs at EL1 (`SPSel = 0`).
- One map-window page (`paging::EL0_PAGE`) can be UXN-clear / PXN for a trampoline or standing payload.
- `ERET` to EL0 switches to a **user TTBR0** (`L1_USER`, ASID=1) that maps kernel text/rodata + the exception stack + the trampoline window (every 2 MiB those ranges occupy), and **omits** `.data` / `.bss` / heap. The lower-EL handler restores kernel TTBR0 from `TPIDR_EL1` before touching kernel data. That path still `TLBI VMALLE1` (kernel `.data` leaves are global).
- Dual EL1 ASIDs (1 vs 2) with `nG` probe pages switch **without** `TLBI VMALLE1` (`src/asid.rs`).
- A bounded **standing** user context on that TTBR0: `SVC #1` stays at EL0 (`el0: standing`), user `MOVZ` runs, `SVC #2` restores EL1 (`el0: restored`). `is_active()` is true only for that lifetime.
- TTBR1 walks are enabled. One kernel-private high page (`TTBR1_PRIV`) is EL1-only; EL0 load faults. Identity RAM is aliased at `va + TTBR1_BASE` via **cloned** RAM tables; EL1 can fetch a real path there and `VBAR_EL1` is the high alias. After a high-VA jump, rustc vtables are rewritten to high aliases (`ident: reloc`) and live identity `.text` after the boot stub is unmapped (`ident: live`), plus the dedicated 16 KiB range (`ident: range` / `ident: ok`). `.rodata` / `.data` / heap stay identity-mapped. `_start` / QEMU `-kernel` stay at `0x4008_0000`. Full identity teardown is Planned.
- Lower-EL AArch64 **sync** handles `SVC` (ADR-013 probes `#0`/`#1`/`#2` plus public ABI `#16`/`#17`/`#18`), a kernel-data IABORT, a kernel-data DABORT, and the TTBR1 private-page DABORT, then returns to EL1t (or stays at EL0 on standing `SVC #1` / `SYS_YIELD` / `SYS_UART_WRITE`). Other lower-EL slots still park ([ADR-004](../03-adr/ADR-004-el1-vbar-brk.md)).
- Public SVC ABI ([syscall.md](syscall.md)): `exit` / `uart_write` / `yield`. A `libctos` CRT wraps those numbers; a hello payload is copied onto the standing EL0 page. ABI + CRT miles only. App hosting Planned.

## Probed miles

| Probe | What closes it | Honesty |
| --- | --- | --- |
| EL0 entered and returned | Serial `el0: svc` + `el0: ok`; `#[test_case]` `el0_svc_roundtrip` | Entered and left. Not a user process. |
| EL0 cannot execute kernel data | Serial `el0: nx kernel`; `#[test_case]` `el0_cannot_execute_kernel_data` | IABORT on `.data` (UXN and/or unmapped). Not isolation. |
| User TTBR0 omits kernel `.data` | Walk `L1_USER`; `#[test_case]` `user_ttbr0_omits_kernel_data` | Distinct user table. Text is still mapped so the handler can run. |
| EL0 cannot *read* kernel `.data` | Serial `el0: no kernel read`; `#[test_case]` `el0_cannot_read_kernel_data` | Translation/permission DABORT. Not PAN. |
| Standing EL0 context | Serial `el0: standing` / `el0: restored`; `#[test_case]` `standing_el0_enter_leave` | Real enter/leave on user TTBR0. `is_active()` true only while standing. Not POSIX. Lower-EL IRQ still parks. |
| ASID field programmed | `user_ttbr0() >> 48 == 1` | Programming fact on the EL0 trampoline. Not the isolation mile. |
| ASID isolation | Serial `asid: dual` / `asid: conflict` / `asid: ok`; `#[test_case]` `asid_isolation_without_vmalle1` | Dual TTBR0 without `TLBI VMALLE1`. Stale ASID-1 data under ASID 2 is Failed. Not “EL0 isolated.” |
| TTBR1 private page | Serial `ttbr1: el1` / `ttbr1: no el0` / `ttbr1: ok`; `#[test_case]` `ttbr1_el1_sees_priv_el0_does_not` | EL1-only high page. Not a relocated kernel. |
| EL1 fetch from TTBR1 high VA | Serial `ttbr1: el1 exec` / `ttbr1: vbar`; `#[test_case]` `el1_executes_from_ttbr1_high_va` | Real EL1 path + high VBAR. Identity boot stub stays. |
| Identity-tear first cut | Serial `ident: split` / `ident: fault` / `ident: high` / `ident: no el0` / `ident: ok`; `#[test_case]` `identity_tear_el1_faults_high_stays` | One identity text page unmapped; high twin still fetches. Not a relocated kernel. Full teardown Planned. |
| Identity text range tear | Serial `ident: jump` / `ident: range` / `ident: text`; `#[test_case]` `identity_text_range_unmapped_boot_stub_stays` | High-VA continuation + 16 KiB dedicated range unmapped; high twins still fetch. Not a relocated kernel. |
| High-VA vtable rewrite + live `.text` tear | Serial `ident: reloc` / `ident: live`; `#[test_case]` `identity_fn_ptrs_rewritten_high` + `live_identity_text_unmapped_boot_stub_stays` | rustc `dyn Write` / fmt tables patched to high aliases; live identity `.text` after `_start` unmapped; `println!` still runs. `.rodata`/`.data`/heap stay. Not a relocated kernel. |
| SVC ABI (`exit` / `uart_write` / `yield`) | Serial `svc: yield` / `svc: user-hi` / `svc: uart` / `svc: exit` / `svc: ok`; `#[test_case]` `el0_svc_abi_yield_uart_exit` + kernel-`.data` / TTBR1-alias reject | Documented numbers 16–18. Not Linux. Not app hosting. |
| `libctos` CRT | Serial `libctos: hi` / `libctos: ok` / `libctos: linked`; `#[test_case]` `libctos_hello_yield_uart_exit` | Linked wrappers, not a hand-encoded trampoline. Not a guest ELF loader. Not app hosting. |

## Still Planned (isolation)

| Probe | What would close it |
| --- | --- |
| PAN | `ID_AA64MMFR1_EL1.PAN != 0` **and** an EL1 access to an EL0-accessible page faults. `-cpu cortex-a57` is ARMv8.0 — usually unimplemented. Do not claim PAN. |
| Identity `.rodata` / `.data` / heap tear | Those identity ranges unmapped; accesses proven high-only. After live `.text` (ADR-020). |
| Full higher-half / identity teardown | The row above plus a guest that no longer fetches identity `.text` after the boot stub ([ADR-020](../03-adr/ADR-020-identity-fnptr-reloc.md) tears live `.text` after a vtable rewrite, not this). |
| EL0 entry without full TLBI | User TTBR0 switch that does not `TLBI VMALLE1` (needs `nG` on kernel `.data` or an ASID-specific invalidate). |
| Lower-EL IRQ while standing | Timer (or other) IRQ taken from EL0 and returned. Still parked. |
| App hosting (Track A A3–A9) | ELF loader, standing EL0 as normal mode, VFS, sample apps ([issue #31](https://github.com/artofdream/ctos/issues/31)). The ABI + CRT miles are not that. |

Unprobed stays **Unknown**. The umbrella isolation row stays **Planned** until PAN + full identity teardown have probes (standing + TTBR1 first cut + EL1 high-VA fetch + torn live identity `.text` + an SVC ABI are not enough). Do not say “EL0 works,” “EL0 isolated,” “app hosting,” or “the kernel moved.”
