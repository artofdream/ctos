# ADR-097 — EL0 app text execute-only (rodata moved to the app-hdr slot)

> **Merge requires sponsor rev 4 written accept.** This change makes one clause of the accepted [ADR-091](ADR-091-el0-isolated-accept-draft.md) revision-3 sentence false as literally worded: “PAN is enabled and EL1 faults on EL0 memory”. The execute-only app text page is EL0 memory (EL0 can execute it), and on the default smoke machine (no FEAT_EPAN) an EL1 load of it does **not** fault with PAN set. ADR-091 is **not** edited here. Its sentence and scope stay exactly as accepted. A candidate revision-4 wording is proposed [below](#proposed-revision-4-wording-for-the-sponsor-not-accepted) for the sponsor to read. Until the sponsor accepts a revision 4 in writing, this PR must not be merged.

- Status: **Proposed** (implementation + evidence; merge blocked on sponsor rev 4 written accept, see above).
- Date: 2026-09-28
- Decision path: sponsor-approved milestone 3 (DSO relay, 2026-09-28): split the app's read-only data out of the app code page so EL0 app text becomes execute-only; $0 (local QEMU + existing GitHub Actions checks).
- Base: **stacked on PR [#149](https://github.com/artofdream/ctos/pull/149)** (ADR-096, branch `isolation/t2t-probe` @ `e6e755a`). It reuses #149's EL0 capture helpers (`arm_el0_capture` / `eret_to_el0_masked` / `el0_capture_result`) and extends the same `scripts/qemu-smoke.sh` leak-build block. Merge #149 first.
- `_start` (`0x4008_0000`) untouched.

## Context

[ADR-094](ADR-094-syscall-pointer-el0-permission.md) closed G4 by making every syscall pointer check require EL0 permission. To keep app string literals printable through `uart_write`, it mapped the app code page (text **and** rodata, one page at `0x80002000`) **EL0 read-only + executable** and said so openly: “An EL0 app can read its own code bytes.” ADR-091's honesty section carries that as a limit.

## Decision

### Layout (every app, same five slots)

| Slot (label unchanged) | VA | Contents | Stage-1 leaf |
| --- | --- | --- | --- |
| `app-hdr` | `0x80000000` | ELF headers **+ `.rodata`** (all string literals and constants) | `AP=11` (EL1 RO, EL0 RO), `UXN=1`, `PXN=1` (unchanged: `map_el0_ro`) |
| `app-text` | `0x80002000` | `.text` only | **`AP=10` (EL1 RO, EL0 none), `UXN=0`, `PXN=1`** (new `map_el0_xo`); was `AP=11` |
| `crt-stack-pan`, `user-stack`, `store-ro` | unchanged | unchanged | unchanged |

- All eight `user/*/linker.ld` place `.rodata` at `0x80000000 + SIZEOF_HEADERS` and start `.text` alone at `0x80002000`. Each app ELF now has exactly two `PT_LOAD`s: `R` at `0x80000000` and `R E` at `0x80002000`. The asm fallback `hello.S` moved its strings to `.rodata`.
- **No sixth slot.** The allowlist, its labels and the at-rest / live inventory lines are byte-identical: `el0-reach: live k=3 u=3 a=0 h=0 pages=6 leaks=0` and `el0-reach: ok allow=app-hdr[0x80000000,0x80001000),app-text[0x80002000,0x80003000),crt-stack-pan[…),user-stack[…),store-ro[…)`.
- `build.rs` enforces it at build time: loads below the text page must be read-only and inside the `0x80000000` page; each app's marker strings must be in that page and **absent** from the text segment.
- `src/loader.rs` enforces it at load time (host unit tests + boot): `Ro` may only map at `0x80000000`, `Exec` only at `0x80002000`, `Rw` never; a page that would combine `Exec` + `Ro` is rejected (`ParseError::RoOnText`, was a union to EL0 RX); anything else is `ParseError::OffSlot`. `Exec` pages map through `map_el0_xo`.
- The A2 `libctos` path maps the hello ro page at the map window with `map_el0_ro` and its text with `map_el0_xo`.

### EL0 reachability (inventory unchanged, XO page still counted)

The ADR-092 walk already defines EL0-reachable as `AP[1]` set **or** `UXN` clear, so the XO page (`AP[1]=0`, `UXN=0`) is counted and must match the `app-text` allowlist entry. `paging::leaf_view` reports `reachable` from the same rule, and a new probe plants an XO leaf (EL0 none, UXN=0) at an unallowlisted VA (`0x80190000`) and requires the walk to catch it (`why=va ap-el0=0 uxn=0`). So the inventory cannot pass by only looking at AP.

### Syscall pointers (ADR-094, strengthened)

`user_range_ok_max(…, Read)` already requires `AP[1]` on every page. A pointer into the XO text now fails it: `uart_write(0x80002000, 8)` from EL0 returns 0 with one permission-check denial, while `uart_write` of the rodata message in `app-hdr` still prints.

### PAN / EPAN facts (probed, not assumed)

- `ID_AA64MMFR1_EL1.PAN` = **2** on QEMU 10.0.13 `-cpu cortex-a76` (FEAT_PAN2, **no FEAT_EPAN**) on the box and in every CI smoke job (`pan: id=2`).
- Without FEAT_EPAN (and `SCTLR_EL1.EPAN`), PAN only blocks EL1 data access to pages that are EL0 **data**-accessible (`AP[1]=1`). The XO page has `AP[1]=0`, so PAN does not apply. Probe: with PAN set (MRS `PAN`=1, and a control load of the EL0-RO `app-hdr` page faulting with `SPSR.PAN`=1 after EL0 trips), an EL1 load of the XO text VA **does not fault** and returns the code word (`xo: el1-pan text va=0x80002000 fault=no match=true mmfr1-pan=2 epan=no`). An EL1 store to it faults, but because `AP[2]=1` (read-only), not because of PAN (`xo: el1-write text … fault=yes intact=true`).
- **Consequence for ADR-091:** the clause “PAN is enabled and EL1 faults on EL0 memory” is not literally true for the XO app text page. Before this change the only standing EL0 pages were `AP[1]=1` pages, so PAN covered all of them (kernel-built probe payload pages mapped EL0-execute-only via `map_el0_exec` are transient and already had the same property). This is why merge needs a rev 4 accept.

### Pre-existing PAN gap found and fixed here (separable)

While probing the above, the live check found that PAN was **not** set while the kernel handled EL0 exceptions or after it returned from an EL0 trip:

1. `SCTLR_EL1.SPAN` was 1 (reset value in QEMU), so taking an exception from EL0 did not set `PSTATE.PAN`, and the handler ran with PAN clear (the ERET SPSR from EL1 has PAN clear too). Evidence before the fix, in the `SYS_YIELD` handler of the real hello app: `sctlr-span=1`, `MRS PAN`=0, and an EL1 load of an EL0-RO page did **not** fault.
2. `return_from_el0` resumed the kernel with SPSR `0x3C4` (PAN bit clear), so after any EL0 trip the kernel ran without PAN until the next `pan::with_user_access` restored it. Evidence: the `app-hdr` control load after EL0 trips did not fault.

ADR-080's `pan: el1-fault` probe runs once at boot, before any EL0 trip, so CI never saw this. It means the accepted clause “PAN is enabled” was not continuously true during syscall handling on `main`; it was true at the boot probe. **Fix:** after a successful PAN enable, clear `SCTLR_EL1.SPAN` (bit 23) so every exception to EL1 sets PAN; `return_from_el0` sets `SPSR.PAN` when PAN is enabled. After the fix: `xo: pan live-syscall pstate-pan=1 sctlr-span=0` (inside the `SYS_YIELD` handler), and the control load faults after EL0 trips. The syscall copy helpers already clear/restore PAN around their copies (`with_user_access`), and the full smoke (all EL0 apps, FAT/net/TCP samples, A9 cross-update) stays green. This part is independent of the XO split and can be split into its own PR if the DSO prefers. It is recorded as a correction row in the honesty ledger.

In the handler, the live check reads `MRS PAN` instead of attempting an EL1 load: once PAN is really set, that load takes a current-EL permission fault inside an exception handler, which the kernel treats as fatal by design. `MRS PAN` is calibrated both ways: it reads 1 in the non-nested control whose load faults with `SPSR.PAN`=1, and it reads 0 in the `pan-keep-leak-probe` build whose control load does not fault. (ADR-080 noted `MRS PAN` “may read 0” under QEMU 10.0.13; that is not reproduced in this build: whenever the fault proved PAN set, `MRS PAN` read 1.)

## Probes and markers (all fail-closed in `scripts/qemu-smoke.sh`)

Live (real `hello-libctos`, inside its `SYS_YIELD` handler, called from the ADR-092 live walk):

- `xo: pan live-syscall pstate-pan=1 sctlr-span=0`
- `xo: live app=hello text va=0x80002000 ap=2 uxn=0 pxn=1 el0=x reach=1 hdr va=0x80000000 ap=3 uxn=1 el0=r rodata=0x80000120 in-text=0 sys-read text=deny rodata=allow` (the app's own `libctos: hi` string is found in the hdr frame and not in the text frame; the ADR-094 read check denies the text VA and allows the rodata VA)

Image probe (an app-shaped ELF built in-kernel, loaded by the real loader, run at EL0 on the user TTBR0):

- `xo: layout loader app-hdr va=0x80000000 ap=3 uxn=1 el0=r app-text va=0x80002000 ap=2 uxn=0 pxn=1 el0=x reach=uxn` — the inventory reports the new layout
- `xo: el0-exec ok entry=0x80002000 el0-rodata va=0x80000800 got=0x584f524441544131` — EL0 executes its text and loads a rodata sentinel from its new location
- `xo: el0-read-text fault va=0x80002000 esr=0x9200000f far=0x80002000 ec=0x24 wnr=0 dfsc=perm-l3 got=0x0` — an EL0 load of its own text takes a lower-EL permission fault
- `xo: el0-uart rodata-str ok` — printed **by EL0** through `uart_write` from a rodata string
- `xo: sys-read rodata ok va=0x80000810 n=27`
- `xo: sys-read text denied sys=uart_write nr=17 va=0x80002000 n=0 checks=1` — a syscall given a text pointer is refused
- `xo: el1-pan control app-hdr va=0x80000800 mrs-pan=1 fault=yes spsr-pan=1 after-el0-trips`
- `xo: el1-pan text va=0x80002000 fault=no match=true mmfr1-pan=2 epan=no` — fact, not a pass condition in the security sense; the smoke pins it so a CPU-model change forces a re-read
- `xo: el1-write text va=0x80002f00 fault=yes intact=true`
- `xo: reach xo-plant caught va=0x80190000 roots=k,u why=va ap-el0=0 uxn=0`
- `xo: ok text=el0-xo rodata=app-hdr el0-read=fault sys-read=deny slots=5`

The smoke requires all thirteen lines (ESR low bits / fault level flexible within permission faults), rejects `xo: leak` / `xo: bad` / `xo: probe missed` / `xo: live missed`, and rejects any `xo: el0-read-text` line with a nonzero `got=`. The same probe runs again as `#[test_case]` `el0_app_text_is_execute_only` under `cargo test` (without the live part). New host unit tests: `loader_plans_hello_as_ro_hdr_and_xo_text`, `loader_rejects_ro_on_text_page`, `loader_rejects_pages_outside_app_slots`.

## Negative tests

- **Build:** the existing `el0-leak-probe` smoke build now also enables `xo-leak-probe` (text mapped EL0 RX again: `AP=11`, `UXN=0`) and `pan-keep-leak-probe` (the pre-fix PAN behaviour). The smoke requires `xo: bad pan live-syscall pstate-pan=0 sctlr-span=1`, `xo: leak live text ap=3 el0=rx sys-read=allow`, `xo: leak layout text ap=3 el0=rx`, `xo: leak el0-read-text va=0x80002000 got=<nonzero code word>`, `xo: leak sys-read text n=8 checks=0`, `xo: bad el1-pan control pan=true mrs-pan=0 fault=false spsr-pan=false` and `xo: probe missed`, and fails if `xo: ok`, `xo: el0-read-text fault` or `xo: sys-read text denied` appears. All the ADR-092/094/096 leak lines are still required from the same build.
- **Tampered logs** (the ADR-097 smoke block run against edited copies of a good serial log; each must fail): `xo: ok` deleted; the read-text fault replaced by a leak line; a translation fault instead of a permission fault; live text shown as `ap=3 el0=rx`; the inventory showing `reach=none`; `xo: probe missed` appended; the text-pointer syscall with `checks=0`; an EPAN CPU (`fault=yes … mmfr1-pan=3 epan=yes`); PAN clear in the handler; and the real leak-build serial. All ten failed with a named missing / forbidden line; the unedited log passed. Results are in the PR body and the honesty ledger.

## ADR-091 clause check (sentence not edited)

| Clause | Still literally true? |
| --- | --- |
| Default smoke machine (QEMU `virt`, `-cpu cortex-a76`, TCG, one core) | Yes (unchanged). |
| Own TTBR0 + own ASID per EL0 task | Yes (unchanged; `asid: ok`, `el0: standing`). |
| EL0 faults when it reads, writes or executes kernel memory | Yes (unchanged markers). |
| Only EL0-reachable pages are five allowlisted user slots backed by non-kernel frames, checked live and at rest | Yes. Still five slots, same labels; the XO page is counted as reachable (`UXN=0`) and allowlisted; live/at-rest lines byte-identical. |
| PAN is enabled and EL1 faults on EL0 memory | **No, not literally, for the XO app text page** (no FEAT_EPAN: EL1 loads of it do not fault). PAN is enabled (and, after this PR's fix, also during syscall handling and after EL0 trips); EL1 still faults on every EL0 **data**-accessible page. |
| Lower-EL IRQ and FIQ taken while EL0 stands | Yes (unchanged). |
| Taken lower-EL SError evidenced by the ADR-090 class | Yes (unchanged). |
| Only identity mapping left in any TTBR0 is the EL1-only `_start` stub page | Yes (unchanged; `ident:` lines). |
| A syscall copies from/to an EL0 pointer only if EL0 may read/write every page | Yes, and stronger for text: EL0 can no longer read its text, so text pointers are refused for copies from EL0. |
| Not speculative / real-hardware / certification | Unchanged. |

## Proposed revision-4 wording for the sponsor (not accepted)

Change only the PAN clause, from “PAN is enabled and EL1 faults on EL0 memory” to:

> “PAN is enabled, including while the kernel handles exceptions from EL0, and EL1 faults on EL0-readable memory (the execute-only app text page is not covered: this CPU has no FEAT_EPAN)”

and replace the limit “EL0 can read its own code bytes” with “app text is EL0 execute-only; EL1 can still read it (no FEAT_EPAN)”. Everything else unchanged. This is a proposal for the sponsor to read, not an edit of ADR-091. If the sponsor declines, the alternative is to close this PR (or keep only the separable PAN fix).

## Limits

- One core, the TCG default smoke machine; not real hardware (no KVM re-run of this change), not side-channel, not a certification.
- Execute-only is enforced for EL0 only. The kernel can still read app text (no FEAT_EPAN, and PAN would not cover it anyway on this CPU). A future CPU with FEAT_EPAN would need `SCTLR_EL1.EPAN` set; the smoke pins `epan=no` so that change is noticed.
- The live check covers `hello-libctos`; the other seven apps are covered by the build-time checks (`build.rs`), the loader rules (host tests) and the fact that they all run in the smoke with the same loader path. The image probe uses an in-kernel app-shaped ELF, not a separate user crate.
- The A9 cross-update to the prior OS image still boots the prior loader; that loader maps `R` segments with `map_el0_ro` and `R E` as execute-only, so the new two-segment ELFs load there too (smoke green), but the prior OS has no ADR-097 probe.
- Writable (`Rw`) app segments are now refused at load time: no allowlisted slot takes them. None of the eight apps has one (their `.data` / `.bss` are empty), so nothing changes today, but an app that needs writable globals would need a design change (it would need a slot, which is an ADR-091 question).
- The PAN fix is proven by the live `MRS PAN` read and the post-trip control fault, not by an EL1 load inside a handler (fatal by design).
