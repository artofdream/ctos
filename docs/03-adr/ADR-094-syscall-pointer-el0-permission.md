# ADR-094 — G4: syscall user pointers require EL0 permission on every page

- Status: Accepted (implementation + evidence). Closes ADR-091 gap **G4** with fail-closed probes that CI gates. The umbrella “EL0 isolated” stays **Planned / non-claim**. ADR-091 stays **Draft**; B3 (the sponsor's written accept) is **not Met**.
- **Follow-on (2026-09-28, M6):** B3 met: the sponsor accepted the ADR-091 revision-3 sentence and scope in writing (2026-09-28 16:41 CEST). [ADR-091](ADR-091-el0-isolated-accept-draft.md) is **Accepted**. “EL0 isolated” is **Verified** only as that sentence, with its limits. Text in this ADR that says the umbrella is Planned / non-claim is as of this ADR's date.
- Date: 2026-09-27
- Decision path: the sponsor did not pick among ADR-091 revision-2 options (a)/(b)/(c). The DSO applied the standing order “close gaps before bringing the sentence back” (relay 2026-09-27 ~12:06 CEST), i.e. option (b): fix G4 first.
- Base: stacked on PR [#144](https://github.com/artofdream/ctos/pull/144) (ADR-092). ADR-093 is the public-identifier guard (PR #145).
- `_start` (`0x4008_0000`) untouched. The stub page is only the *target* of a refused read. No code maps, writes or changes it.

## Context

ADR-092 found **G4** by code reading. `syscall::user_range_ok_max` accepted any page with a valid leaf in the user TTBR0. `copy_user` then cleared PAN and loaded the bytes. EL0 could therefore ask the kernel to do what EL0 itself cannot:
- **Read:** `SYS_UART_WRITE` could copy the EL1-only `_start` stub page, or an EL1-only map-window page present while the app runs, to the UART.
- **Write:** `copy_to_user` (`SYS_FS_READ`, `SYS_NET_MAC`) could write EL0's own execute-only text.

That is a confused-deputy path around the page-table boundary. So ADR-091 revision 2 limited its sentence to direct EL0 access.

## Decision

### 1. The check (`src/syscall.rs`)

- `user_range_ok_max(ptr, len, max, access)` gains an access kind:
  - `UAccess::Read`: the kernel copies from EL0 (`copy_user_max`: `uart_write`, `fs_write`, and paths for `fs_create` / `fs_open` / `fs_mkdir`).
  - `UAccess::Write`: the kernel copies to EL0 (`copy_to_user`, plus the pre-checks in `SYS_FS_READ` and `SYS_NET_MAC`).
- The existing guards stay: length cap, overflow, identity/canonical, and “mapped in user TTBR0 and kernel TTBR0”.
- New, for **every** 4 KiB page in the range: `paging::user_el0_access(page)` reads the user-root leaf's AP bits.
  - A read needs EL0-readable: `AP[1]=1`.
  - A write needs EL0-writable: `AP[1]=1` and `AP[2]=0`.
  - Execute-only (`AP[2:1]=00`, UXN clear), kernel-only and unmapped pages fail.
- On failure the syscall returns its existing error value, and no byte moves:
  - `uart_write` / `fs_write` / `net_mac`: 0
  - `fs_create` / `fs_open`: fd 0
  - `fs_mkdir`: `FS_MKDIR_BAD_PATH`
  - `fs_read`: `FS_ERR`
- The map-window L3 is shared by the kernel and user roots. So the leaf the check reads is the leaf the copy uses under kernel TTBR0.
- A counter, `SYSPTR_DENIED`, counts refusals made by the permission check itself. The probes require a delta of exactly 1 per call. So a refusal cannot come from some other guard (length, unmapped) by accident.

### 2. Loader: app rodata EL0-readable (`src/loader.rs`, `src/libctos.rs`, `src/paging.rs`)

- Apps link `.text` and `.rodata` into one R+X `PT_LOAD` page. `Perm::Exec` used to map it EL0 **execute-only** (`l3_page_el0_exec`, AP=00). The app's own strings therefore failed the new read rule.
- `Perm::Exec` now maps with the new `paging::map_el0_text` / `l3_page_el0_text`:
  - AP[2:1]=11: EL0 read-only, EL1 read-only
  - UXN clear, PXN set
  - so EL0 can read and execute the page but not write it.
- Bytes are written through the frame's TTBR1 alias, as before. Cache maintenance by the EL0 VA runs inside `pan::with_user_access`, since the page is EL0-accessible now.
- The A2 `libctos::run_hello` path does the same.
- **No new slot, no split page, no app ELF change.** The app image and its sha256 are byte-identical, so the A9 cross-update to the prior OS still boots the same ELF.
- The G1 walk still passes. The page was already EL0-reachable (UXN clear), and it is not EL0 W+X.
- Kernel-built probe payloads keep execute-only code pages. Their strings and buffers move to a new EL0-RW NX data page, `syscall::EL0_DATA_VA` (window slot 3, the allowlisted `crt-stack-pan` slot).

### 3. Probes (`syscall::observe_sys_ptr_probe`, called from `kernel_main_high` after `svc: ok`)

| Line | What EL0 asks | Must be |
| --- | --- | --- |
| `el0: sys-ptr control rw-data n=6` | `SYS_NET_MAC` into the EL0-RW data page | 6 bytes land, equal to the guest MAC (positive control: the NIC works, so a later 0 is the check) |
| `el0: sys-ptr denied kernel-stub` | `SYS_UART_WRITE(0x40080000, 8)`: the `_start` stub page | returns 0, nothing printed, 1 check refusal |
| `el0: sys-ptr denied kernel-window` | `SYS_UART_WRITE` of a kernel-only map-window page (`map_page`, EL1-RW, EL0-none, slot 11) | returns 0, nothing printed, 1 check refusal |
| `el0: sys-ptr denied xo-text` | `SYS_NET_MAC` writing its own execute-only code page (+0x800, sentinel) | returns 0, sentinel unchanged, 1 check refusal |
| `el0: sys-ptr denied write-ro` | `SYS_NET_MAC` writing an EL0-RO page (the `store-ro` slot, sentinel set via alias) | returns 0, sentinel unchanged, 1 check refusal |
| `el0: sys-ptr denied straddle` | `SYS_UART_WRITE(data+4092, 8)`: EL0-RW page, then a kernel-only page | returns 0, nothing printed, 1 check refusal (the second page is checked) |
| `el0: sys-ptr denied sys=<name> nr=<n> {read kernel-stub \| write xo-text}` ×7 | every pointer-taking syscall: `uart_write` 17, `fs_create` 19, `fs_open` 20, `fs_mkdir` 27, `fs_write` 22 (reads of the stub page); `fs_read` 21, `net_mac` 24 (writes of own xo text) | each returns its error value, 1 check refusal |
| `el0: sys-ptr sweep syscalls=7 denied=7` | summary | all 7 |
| `el0: sys-ptr ok` | all of the above | only then |

- **Fail-closed:** the smoke requires all 15 lines. It rejects `el0: sys-ptr leaked`, `el0: sys-ptr bad`, `el0: sys-ptr skip` and `el0: sys-ptr probe missed`.
- **Leak-probe build:** the `sysptr-leak-probe` feature (built together with `reach-leak-probe,write-leak-probe` into `target/el0-leak-probe`) makes the permission check always pass, keeping only the old guards. That kernel must print:
  - `el0: sys-ptr leaked kernel-stub n=8`: 8 bytes of the stub page go to the UART
  - `el0: sys-ptr leaked kernel-window n=8`
  - `el0: sys-ptr leaked xo-text n=6 intact=false checks=0`: the kernel wrote EL0's execute-only text
  - `el0: sys-ptr skip write-ro`: an EL1 store to an AP[2]=1 page would take an EL1 permission fault, so the probe stops there
  - no `el0: sys-ptr ok` and no `el0: sys-ptr denied`
  
  The smoke requires those and fails otherwise.
- `cargo test` adds `sys_ptr_requires_el0_permission` and `abi_success_from_el0_data_page`.

### 4. G1 allowlist

**Unchanged: five slots, fail-closed** (`app-hdr`, `app-text`, `crt-stack-pan`, `user-stack`, `store-ro`). No new EL0-reachable slot:
- The EL0 data page reuses `crt-stack-pan`.
- The write-ro target reuses `store-ro`.
- The kernel-window (slot 11) and straddle (slot 4) pages are EL0-none, so they are not EL0-reachable.
- All probe pages are unmapped before the walk's steady pass.

`el0-reach: live k=3 u=3 a=0 h=0 pages=6 leaks=0` and `el0-reach: ok allow=…` are byte-identical to ADR-092.

## Consequences

- G4 is closed on the default smoke machine: no syscall that takes a user pointer can read a page EL0 cannot read, or write a page EL0 cannot write. Probes cover each such syscall. The ADR-091 draft drops its “direct EL0 memory access only” limit (revision 3). Only **G3** (the optional B2-P re-verify) is left.
- App text is now EL0-*readable* as well as executable. That is the usual Unix-like model for a shared text+rodata page, and still not writable. An EL0 app can read its own code bytes. That is not a kernel-isolation property.
- **Not covered (stated, not gaps in G4's scope):**
  - TOCTOU between check and copy. Today there is one core, and nothing changes the calling task's user page tables during a syscall. On SMP this would need `AT S1E0R/W` or a fault-fixup copy.
  - `AT`-based checks.
  - Real hardware. `LDTR`/`STTR` unprivileged loads/stores would enforce the same rule in hardware; that is a possible later hardening.
- Still **not** “EL0 isolated”, not side-channel, not real hardware, not certified.

## Honesty

Say: “ADR-094 closes G4: syscall user pointers need EL0 read/write permission on every page, proven by refused kernel-stub / kernel-window reads, refused xo-text / RO writes, a straddle case, a seven-syscall sweep, and a leak-probe build that must fail.” Do not say “EL0 isolated”, “secure” or “B3 met”.
