# EL0 isolation (P-SEC-3 / ADR-013)

**Isolation: Planned.** A first mile, a user-TTBR0 read mile, an ASID TLB mile, a standing EL0 context, a TTBR1 private-page first cut, an EL1 high-VA fetch mile, an identity-tear first cut, an identity `.text` range tear, a live identity `.text` tear after a high-VA vtable rewrite, an identity `.rodata` tear, a PAN ID-field probe (enable Planned), a **documented SVC ABI mile**, a **`libctos` CRT mile**, a **guest ELF PT_LOAD loader mile**, a **standing-task (normal mode) mile**, a **thin VFS + memfs mile**, a **virtio-blk + FAT16 mile**, and an **OS/app slot first cut** exist. Those miles are not “app hosting is done.” A8 is documented rebuild recipes ([what-can-run.md](../overview/what-can-run.md)). A9 is two artifacts + FAT `/hello` ([ADR-030](../03-adr/ADR-030-os-app-slots.md)); cross-update stays Planned. Do not claim userspace, app hosting, or “EL0 isolated.” Standing enter/leave as a guest sample: [apps-today.md](apps-today.md). CRT + loader + standing-as-normal: [building-or-porting.md](building-or-porting.md).

Direction: [ADR-013](../03-adr/ADR-013-el0-isolation-direction.md). TTBR1 first cut: [ADR-016](../03-adr/ADR-016-ttbr1-private-page.md). EL1 fetch mile: [ADR-017](../03-adr/ADR-017-ttbr1-high-el1-exec.md). Identity-tear first cut: [ADR-018](../03-adr/ADR-018-identity-teardown-first-cut.md). Identity `.text` range tear: [ADR-019](../03-adr/ADR-019-identity-text-range-tear.md). Live `.text` tear: [ADR-020](../03-adr/ADR-020-identity-fnptr-reloc.md). Identity `.rodata` tear: [ADR-025](../03-adr/ADR-025-identity-rodata-tear.md). Identity `.data` tear: [ADR-037](../03-adr/ADR-037-identity-data-tear.md). PAN capability: [ADR-026](../03-adr/ADR-026-pan-capability.md). SVC ABI: [ADR-021](../03-adr/ADR-021-svc-syscall-abi.md) / [syscall.md](syscall.md). CRT / `libctos`: [ADR-022](../03-adr/ADR-022-libctos-crt.md). Guest loader: [ADR-023](../03-adr/ADR-023-elf-pt-load-loader.md). Standing-as-normal: [ADR-024](../03-adr/ADR-024-standing-el0-normal.md). Thin VFS + memfs: [ADR-027](../03-adr/ADR-027-thin-vfs-memfs.md). Threat model: [security.md](security.md). Code: `src/el0.rs`, `src/syscall.rs`, `src/libctos.rs`, `src/loader.rs`, `src/vfs.rs`, `libctos/`, `src/asid.rs`, `src/ttbr1.rs`, `src/teardown.rs`, `src/pan.rs`.

## What exists today

- Kernel runs at EL1 (`SPSel = 0`).
- One map-window page (`paging::EL0_PAGE`) can be UXN-clear / PXN for a trampoline or standing payload.
- `ERET` to EL0 switches to a **user TTBR0** (`L1_USER`, ASID=1) that maps kernel text/rodata + the exception stack + the trampoline window (every 2 MiB those ranges occupy), and **omits** `.data` / `.bss` / heap. The lower-EL handler restores kernel TTBR0 from `TPIDR_EL1` before touching kernel data. That path still `TLBI VMALLE1` (kernel `.data` leaves are global).
- Dual EL1 ASIDs (1 vs 2) with `nG` probe pages switch **without** `TLBI VMALLE1` (`src/asid.rs`).
- A bounded **standing** user context on that TTBR0: `SVC #1` stays at EL0 (`el0: standing`), user `MOVZ` runs, `SVC #2` restores EL1 (`el0: restored`). `is_active()` is true only for that lifetime. That dual-SVC is the ADR-013 probe.
- A **standing task** (A4 / ADR-024) is the supported path for a loaded freestanding image: the A3 loader maps `PT_LOAD`, `is_active()` is true while it runs, `yield` / `uart_write` stay at EL0 (`el0: task-active`), `exit` restores (`el0: task-exit` / `el0: task-restored`). An unexpected lower-EL sync restores fail-closed (`el0: restore-fail`) instead of parking. That is the ctos process model ([ADR-035](../03-adr/ADR-035-process-model-standing-el0.md)) — not Linux `fork` / `execve` / `wait`.
- TTBR1 walks are enabled. One kernel-private high page (`TTBR1_PRIV`) is EL1-only; EL0 load faults. Identity RAM is aliased at `va + TTBR1_BASE` via **cloned** RAM tables; EL1 can fetch a real path there and `VBAR_EL1` is the high alias. After a high-VA jump, rustc vtables are rewritten to high aliases (`ident: reloc`) and live identity `.text` after the boot stub is unmapped (`ident: live`), plus the dedicated 16 KiB range (`ident: range` / `ident: ok`). Identity `.rodata` is unmapped after a pointer rewrite (`ident: rodata` / `ident: rodata-fault` / `ident: rodata-high`). `.data` / heap stay identity-mapped. `_start` / QEMU `-kernel` stay at `0x4008_0000`. Full identity teardown (`.data`/heap) is Planned. PAN ID field is printed (`pan: absent` on `-cpu cortex-a57`); PAN enable is Planned.
- Lower-EL AArch64 **sync** handles `SVC` (ADR-013 probes `#0`/`#1`/`#2` plus public ABI `#16`–`#23`), a kernel-data IABORT, a kernel-data DABORT, and the TTBR1 private-page DABORT, then returns to EL1t (or stays at EL0 on standing `SVC #1` / `SYS_YIELD` / `SYS_UART_WRITE` / memfs SVCs). Other lower-EL slots still park ([ADR-004](../03-adr/ADR-004-el1-vbar-brk.md)).
- Public SVC ABI ([syscall.md](syscall.md)): `exit` / `uart_write` / `yield` plus memfs `fs_*` (19–23). A `libctos` CRT wraps those numbers; a hello payload is copied onto the standing EL0 page (A2) and also parsed as ELF64 `PT_LOAD` on the guest (A3). A4 runs that loaded image as a standing task until `exit`. A6 is in-RAM memfs. ABI + CRT + loader + standing-task + memfs miles. App hosting Planned.

## Probed miles

| Probe | What closes it | Honesty |
| --- | --- | --- |
| EL0 entered and returned | Serial `el0: svc` + `el0: ok`; `#[test_case]` `el0_svc_roundtrip` | Entered and left. Not a user process. |
| EL0 cannot execute kernel data | Serial `el0: nx kernel`; `#[test_case]` `el0_cannot_execute_kernel_data` | IABORT on `.data` (UXN and/or unmapped). Not isolation. |
| User TTBR0 omits kernel `.data` | Walk `L1_USER`; `#[test_case]` `user_ttbr0_omits_kernel_data` | Distinct user table. Text is still mapped so the handler can run. |
| EL0 cannot *read* kernel `.data` | Serial `el0: no kernel read`; `#[test_case]` `el0_cannot_read_kernel_data` | Translation/permission DABORT. Not PAN. |
| Standing EL0 context | Serial `el0: standing` / `el0: restored`; `#[test_case]` `standing_el0_enter_leave` | Real enter/leave on user TTBR0. `is_active()` true only while standing. Not POSIX. Lower-EL IRQ still parks. |
| Standing EL0 as normal mode | Serial `el0: task-enter` / `el0: task-active` / `el0: task-exit` / `el0: task-restored` / `el0: restore-fail` / `el0: task-ok`; `#[test_case]` `standing_task_runs_until_exit` + `standing_task_fault_restores` | Loaded A3 image until `exit`. Fail-closed restore. Not POSIX. Not isolation. |
| ASID field programmed | `user_ttbr0() >> 48 == 1` | Programming fact on the EL0 trampoline. Not the isolation mile. |
| ASID isolation | Serial `asid: dual` / `asid: conflict` / `asid: ok`; `#[test_case]` `asid_isolation_without_vmalle1` | Dual TTBR0 without `TLBI VMALLE1`. Stale ASID-1 data under ASID 2 is Failed. Not “EL0 isolated.” |
| TTBR1 private page | Serial `ttbr1: el1` / `ttbr1: no el0` / `ttbr1: ok`; `#[test_case]` `ttbr1_el1_sees_priv_el0_does_not` | EL1-only high page. Not a relocated kernel. |
| EL1 fetch from TTBR1 high VA | Serial `ttbr1: el1 exec` / `ttbr1: vbar`; `#[test_case]` `el1_executes_from_ttbr1_high_va` | Real EL1 path + high VBAR. Identity boot stub stays. |
| Identity-tear first cut | Serial `ident: split` / `ident: fault` / `ident: high` / `ident: no el0` / `ident: ok`; `#[test_case]` `identity_tear_el1_faults_high_stays` | One identity text page unmapped; high twin still fetches. Not a relocated kernel. Full teardown Planned. |
| Identity text range tear | Serial `ident: jump` / `ident: range` / `ident: text`; `#[test_case]` `identity_text_range_unmapped_boot_stub_stays` | High-VA continuation + 16 KiB dedicated range unmapped; high twins still fetch. Not a relocated kernel. |
| High-VA vtable rewrite + live `.text` tear | Serial `ident: reloc` / `ident: live`; `#[test_case]` `identity_fn_ptrs_rewritten_high` + `live_identity_text_unmapped_boot_stub_stays` | rustc `dyn Write` / fmt tables patched to high aliases; live identity `.text` after `_start` unmapped; `println!` still runs. Not a relocated kernel. |
| Identity `.rodata` tear | Serial `ident: ro-reloc` / `ident: rodata` / `ident: rodata-fault` / `ident: rodata-high`; `#[test_case]` `identity_rodata_unmapped_high_stays` | Identity `.rodata` unmapped; high twin still loads. `.data`/heap stay. Not a relocated kernel. |
| PAN ID field | Serial `pan: id=` / `pan: absent`; `#[test_case]` `pan_unimplemented_on_probe_cpu` | `ID_AA64MMFR1_EL1.PAN == 0` on `-cpu cortex-a57`. Enable stays Planned. Not PAN. |
| SVC ABI (`exit` / `uart_write` / `yield`) | Serial `svc: yield` / `svc: user-hi` / `svc: uart` / `svc: exit` / `svc: ok`; `#[test_case]` `el0_svc_abi_yield_uart_exit` + kernel-`.data` / TTBR1-alias reject | Documented numbers 16–18. Not Linux. Not app hosting. |
| `libctos` CRT | Serial `libctos: hi` / `libctos: ok` / `libctos: linked`; `#[test_case]` `libctos_hello_yield_uart_exit` | Linked wrappers, not a hand-encoded trampoline. Not app hosting. |
| Guest ELF PT_LOAD loader | Serial `loader: mapped` / `loader: ok`; `#[test_case]` `loader_maps_hello_and_erets` | Guest parse + map into user TTBR0 + `ERET` to `e_entry`. Not a Linux ABI. Not app hosting. |
| Thin VFS + memfs | Serial `fs: create` / `fs: write` / `fs: read` / `fs: el0` / `fs: ok`; `#[test_case]` `vfs_memfs_create_write_read_close` + `el0_fs_svc_roundtrip` | In-RAM named buffers. Not POSIX. Not FAT. Not app hosting. |
| OS/app slot first cut | Serial `slot: fat` / `slot: mapped` / `slot: ok`; `#[test_case]` `slot_load_from_fat_erets` | A9 ([ADR-030](../03-adr/ADR-030-os-app-slots.md)). FAT `/hello` + A3 map. A2–A4 still embed. Not cross-update. Not app hosting. |

## Still Planned (isolation)

| Probe | What would close it |
| --- | --- |
| PAN enable | `ID_AA64MMFR1_EL1.PAN != 0` **and** an EL1 access to an EL0-accessible page faults. `-cpu cortex-a57` is ARMv8.0 — ID field is 0 (`pan: absent`, ADR-026). Do not claim PAN. Do not switch `-cpu`. |
| Identity `.data` / stack tear | `.data`/`.bss`/linker stacks unmapped; SP high-only ([ADR-037](../03-adr/ADR-037-identity-data-tear.md)). |
| Identity heap tear | Heap unmapped; allocator returns high VAs. After ADR-037. |
| Full higher-half / identity teardown | Heap tear plus a guest that no longer fetches identity `.text` after the boot stub ([ADR-020](../03-adr/ADR-020-identity-fnptr-reloc.md) / [ADR-025](../03-adr/ADR-025-identity-rodata-tear.md) / [ADR-037](../03-adr/ADR-037-identity-data-tear.md)). |
| EL0 entry without full TLBI | User TTBR0 switch that does not `TLBI VMALLE1` (needs `nG` on kernel `.data` or an ASID-specific invalidate). |
| Lower-EL IRQ while standing | Timer (or other) IRQ taken from EL0 and returned. Still parked. |
| A9 cross-update / product app hosting | Same app ELF on OS n and n+1; kernel without the A2–A4 embed. |

Unprobed stays **Unknown**. The umbrella isolation row stays **Planned** until PAN **enable** + full identity teardown (`.data`/heap) have probes (standing + TTBR1 first cut + EL1 high-VA fetch + torn live identity `.text` + torn `.rodata` + a PAN ID-field print + an SVC ABI are not enough). Do not say “EL0 works,” “EL0 isolated,” “app hosting,” or “the kernel moved.”
