# ADR-092 — G1 EL0-reachability walk + G2 EL0 write permission fault (sponsor option (b) on ADR-091)

- Status: Accepted (implementation + evidence). Closes ADR-091 gaps **G1** and **G2** with fail-closed probes that CI gates. Found a new gap, **G4** (syscall copy helpers; below), which is recorded, not fixed. The umbrella “EL0 isolated” stays **Planned / non-claim**. ADR-091 stays **Draft**; B3 (the sponsor's written accept) is **not Met**.
- **Follow-on (2026-09-28, M6):** B3 met: the sponsor accepted the ADR-091 revision-3 sentence and scope in writing (2026-09-28 16:41 CEST). [ADR-091](ADR-091-el0-isolated-accept-draft.md) is **Accepted**. “EL0 isolated” is **Verified** only as that sentence, with its limits. Text in this ADR that says the umbrella is Planned / non-claim is as of this ADR's date.
- Date: 2026-09-27
- Sponsor decision: 2026-09-27 00:09 CEST, option (b) on ADR-091: “Close G1 and G2 first (full EL0-reachable walk plus an EL0 write test), then bring the sentence back to me.”
- Base: stacked on PR [#143](https://github.com/artofdream/ctos/pull/143) (ADR-091 draft).
- `_start` (`0x4008_0000`) untouched: no probe targets it, and the stub page stays the ADR-089 exception.

## Context

ADR-091 rested its clause “EL0 faults when it reads or executes kernel memory” on two single-address probes plus a code-reading note. It listed **G1** (no exhaustive EL0-permission walk) and **G2** (no EL0 write probe). The sponsor asked for both before re-reading the sentence.

## Decision

### G1 — fail-closed EL0-reachability walk (`paging::el0_reach`, `src/reach.rs`)

1. **What is walked:** every valid leaf of four translation roots:
   - `k`: kernel TTBR0
   - `u`: user TTBR0
   - `a`: ASID-B TTBR0
   - `h`: **TTBR1**, added beyond the request because EL0 can walk TTBR1 while `TCR_EL1.EPD1 = 0`
2. **EL0-reachable** = `AP[1]` set (EL0 load/store) **or** `UXN` clear (EL0 fetch). Table-level `APTable`/`UXNTable` bits are ignored. That can only over-count, and this kernel never sets them.
3. **Rules per coalesced run.** Any failure is a leak, printed `el0-reach: leak … why=<rule>`:
   - `ttbr1`: no EL0-reachable leaf in TTBR1 at all.
   - `device`: never Device memory.
   - `kernel-pa`: backed only by frame-pool frames, `[pool_start, pool_end)`. Never the image, linker stacks or static page tables below `pool_start`, and never outside RAM.
   - `heap-pa`: never the kernel heap run.
   - `el0-wx`: never EL0-writable and EL0-executable at once.
   - `va`: inside one `EL0_ALLOW` range.
4. **`EL0_ALLOW`:** the five 4 KiB map-window slots this kernel hands to EL0, taken from every `map_el0_*` call site:

   | Name | VA | Used by |
   | --- | --- | --- |
   | `app-hdr` | `[0x8000_0000, 0x8000_1000)` | loader: rust-lld ELF-header `PT_LOAD` (EL0-RO) |
   | `app-text` | `[0x8000_2000, 0x8000_3000)` | `EL0_PAGE`: loaded app text + rodata (EL0 execute-only), EL0 probe trampolines |
   | `crt-stack-pan` | `[0x8000_3000, 0x8000_4000)` | libctos CRT stack; PAN probe page |
   | `user-stack` | `[0x8000_7000, 0x8000_8000)` | loader user stack (EL0-RW, NX) |
   | `store-ro` | `[0x8000_9000, 0x8000_a000)` | G2 read-only store target |

   The rest of the 2 MiB window is **not** allowed. A larger guest image needs a new ADR, not a silent widening.
5. **Three views:**
   - **Live:** armed in the loader probe and taken on hello-libctos's first `SYS_YIELD`, while the app stands at EL0 with every user page mapped.
   - **Steady:** after the boot probes. Nothing may be EL0-reachable.
   - **Negative:** four plants (never accessed; removed, then `TLBI VAAE1`). Each must be caught in the expected roots under the expected rule, and a clean walk must follow:
     - `kernel-pa`: EL0-RO mapping of a kernel `.data` page at the allowlisted `0x8000_7000`
     - `va`: pool frame at off-allowlist `0x8019_0000`
     - `ttbr1`: pool frame at `TTBR1_BASE + 1 MiB`
     - `el0-wx`: EL0 RW+X pool frame at `0x8000_2000`
6. **Leak build** (`--features reach-leak-probe`): a persistent EL0-RO mapping of kernel `.data` at `0x8000_7000` before the steady walk. It must print `el0-reach: leak k|u … why=kernel-pa` and withhold `el0-reach: ok`.

### G2 — EL0 write permission fault (`el0::el0_write_faults`)

1. EL0 runs `MOVZ X1, #magic; STR X1, [X0]; BRK #0` with `X0 = target`. The store must take a **lower-EL data abort** with:
   - `EC = 0x24`
   - `WnR = 1`
   - `DFSC` = permission fault, level 1, 2 or 3
   - `FAR = target`
   - `ELR` = the `STR`
   - the target unchanged afterwards

   Reaching the `BRK` means the store completed, and the probe prints `el0: write-succeeded`. Any other exception prints `el0: write-bad`. Either one withholds `el0: write-ok`, and the smoke rejects both.
2. **Targets:**
   - (a) kernel `.data` through its **TTBR1** alias: mapped EL1-RW / EL0-none, and TTBR1 walks stay live while EL0 runs.
   - (b) an **EL0-read-only user page** at `0x8000_9000`, filled through its TTBR1 RAM alias because PAN blocks EL1 access through the EL0 mapping.
   - `_start` is deliberately not a target.
3. **Negative build** (`--features write-leak-probe`): target (b) is mapped EL0-RW, so the store completes. The probe must print `el0: write-succeeded user-ro` and withhold `el0: write-ok`. The smoke builds both leak features in one kernel (`target/el0-leak-probe`).

## Evidence (agent box, 2026-09-27 00:20–00:23 CEST; QEMU 10.0.13 Debian, `virt -cpu cortex-a76` TCG, rustc 1.100.0-nightly `0fc141305`): `qemu-smoke: ok`

```
el0: write-fault kernel va=0xffffff8040201008 esr=0x9200004f far=0xffffff8040201008 ec=0x24 wnr=1 dfsc=perm-l3
el0: write-fault user-ro va=0x80009000 esr=0x9200004f far=0x80009000 ec=0x24 wnr=1 dfsc=perm-l3
el0: write-ok kernel,user-ro
el0-reach: range u lo=0x80000000 hi=0x80001000 pa=0x4025d000 mem el0-ro el0-nx
el0-reach: range u lo=0x80002000 hi=0x80003000 pa=0x4026e000 mem el0-none el0-x
el0-reach: range u lo=0x80007000 hi=0x80008000 pa=0x4026f000 mem el0-rw el0-nx
el0-reach: live k=3 u=3 a=0 h=0 pages=6 leaks=0
el0-reach: steady k=0 u=0 a=0 h=0 pages=0 leaks=0
el0-reach: neg kernel-pa caught va=0x80007000 roots=k,u why=kernel-pa
el0-reach: neg va caught va=0x80190000 roots=k,u why=va
el0-reach: neg ttbr1 caught va=0xffffff8000100000 roots=h why=ttbr1
el0-reach: neg el0-wx caught va=0x80002000 roots=k,u why=el0-wx
el0-reach: neg clean
el0-reach: ok allow=app-hdr[0x80000000,0x80001000),app-text[0x80002000,0x80003000),crt-stack-pan[0x80003000,0x80004000),user-stack[0x80007000,0x80008000),store-ro[0x80009000,0x8000a000)
```

(The `live` walk also prints the same three `range k …` lines; `k` and `u` share the map-window L3.) Leak build: `el0: write-succeeded user-ro va=0x80009000 esr=0xf2000000` (the BRK), `el0-reach: leak-probe planted va=0x80007000`, `el0-reach: leak k|u lo=0x80007000 hi=0x80008000 pa=0x40204000 mem el0-ro el0-nx why=kernel-pa`, `el0-reach: steady k=1 u=1 a=0 h=0 pages=2 leaks=2` → `qemu-smoke: el0-leak-probe caught (fail-closed ok)`. `cargo test`: new cases `el0_store_to_kernel_and_ro_page_faults` and `el0_reach_rest_clean_and_plants_caught` pass (146 `[ok]`). CI: see the ledger row for this PR.

## Surprises / findings

1. **Nothing is EL0-reachable at rest.** Every probe and loader run unmaps its user pages, so a steady-state walk alone would never exercise the allowlist. Hence the live walk from inside a standing app.
2. **Only one existing read probe was a permission fault.** `el0: no kernel read` targets the *torn identity* alias of kernel `.data`, so it is a **translation** fault. `ttbr1: no el0` (an EL0 load of the TTBR1 private page) was already a permission-class check on a mapped kernel page. G2(a) is the first **store** on a mapped kernel page.
3. **ADR-091's code-reading note was incomplete.** It said only `l3_page_el0_rw` / `l3_page_el0_ro` set EL0 access and missed `l3_page_el0_exec` (AP = 00, **UXN clear**: EL0 execute-only). The walk covers both `AP[1]` and `UXN`, so the note is superseded.
4. **App rodata sits on an EL0 execute-only page.** The loader unions hello's R segment into the RX page, so EL0 cannot load its own rodata directly. The apps still work because the kernel reads their strings in syscalls (G4).
5. `a = 0` and `h = 0`: ASID-B's window pages are EL1-only nG probe pages. The kernel TTBR0 carries the same EL0 pages as user TTBR0 (shared window L3), and EL0 only ever runs on user TTBR0.

## New gap G4 (Documented by code reading; not probed, not fixed)

`syscall::user_range_ok_max` (used by `copy_user` / `copy_to_user` / `path_from_user`) accepts a user pointer if `paging::user_mapped(page) && paging::is_mapped(page)`. `user_mapped` accepts **any valid leaf in user TTBR0, whatever its AP bits**. The copy then clears `PSTATE.PAN`. Consequences:

- `SYS_UART_WRITE(0x4008_0000, n)` from EL0 would copy bytes of the EL1-only `_start` stub page to the UART.
- Any EL1-only map-window page that exists while an app runs would be readable the same way, because the window L3 is shared by kernel and user TTBR0.
- `copy_to_user` could write an EL0 execute-only page (the app's own text).

The direct EL0 access path is closed (G1/G2); the **kernel-mediated** path is not. The fix is to require EL0 read/write permission on the user leaf (or use `AT S1E0R/W`). It needs the loader to map app rodata EL0-readable (AP = 11 with UXN clear, i.e. EL0 R+X), which changes loader and trampoline permissions, so it is out of this milestone's scope. ADR-091 lists G4 for the sponsor.

## Honesty

Say: “ADR-092: a fail-closed walk of kernel, user and ASID-B TTBR0 and TTBR1 finds EL0-reachable pages only in five allowlisted user slots backed by non-kernel frames (live: 3 per shared window root while an app stands; at rest: none), and four planted leaks are caught; EL0 stores to kernel data and to an EL0-RO page take permission faults (ESR/FAR checked). Syscall copy helpers do not yet check EL0 permission (G4).” Do **not** say “EL0 isolated”, “EL0 cannot obtain kernel bytes by any path”, or “exhaustively verified” beyond these four roots on this machine.

## Consequences

- Code:
  - `src/paging.rs`: `el0_reach`, `EL0_ALLOW`, plant helpers, `EL0_STORE_RO_VA`.
  - `src/reach.rs` (new).
  - `src/exception.rs`: store-probe capture.
  - `src/el0.rs`: G2.
  - `src/syscall.rs` / `src/loader.rs`: live-walk hook.
  - `src/main.rs`.
  - `Cargo.toml`: `reach-leak-probe`, `write-leak-probe`.
  - `scripts/qemu-smoke.sh`: required markers, rejects, el0-leak build.
- Docs: ADR-091 revision 2 (G1/G2 closed, G4 added, sentence restated), ADR-055 §A rows, ledger, roadmap, threat model **v1.67**, el0.md, SUMMARY, MOC, daily brief, session memory.
