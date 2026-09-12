# ADR-031 — Track A leftovers: cross-update, FAT-only hello, isolation honesty

- Status: Accepted (leftover mile when the serial / host smoke / tests pass)
- Date: 2026-09-12

## Context

Track A children A1–A9 have first cuts. After [ADR-030](ADR-030-os-app-slots.md) merged on `main` `ba6541c`, three honesty gaps remained:

1. **A9 cross-update.** Same published `hello-libctos.elf` on this OS **and** a documented prior OS. ABI stability is not that probe.
2. **Embed honesty.** A2–A4 still `include_bytes!` a hello copy while A9 loaded FAT `/hello`. Dual path or remove the embed.
3. **Isolation leftovers.** After A5: identity `.data` / heap tear if safe; PAN enable only if `ID_AA64MMFR1_EL1.PAN != 0` on default `-cpu cortex-a57`. Do not claim “EL0 isolated.”

No new FR/NFR IDs. Author does not merge ([ADR-002](ADR-002-pr-identity-split.md)).

## Hypotheses

| ID | Option | Notes |
| --- | --- | --- |
| **H1 (chosen)** | Cross-update: build documented prior OS `ba6541c` (A9 merge — earliest `main` tip with the slot/FAT path) and boot the **same** published app ELF on that kernel and on this tip. Fail closed. | Real SHA. No stored kernel blob. |
| **H2 (rejected)** | Claim cross-update from unchanged SVC numbers, or invent a prior kernel artifact that was never built here. | Invents evidence. |
| **H3 (chosen)** | Remove `include_bytes!` of hello. A2 flattens FAT `/hello` `PT_LOAD`; A3/A4 parse the same file. `build.rs` still compiles and publishes the payload. Host smoke greps that `src/` has no hello `include_bytes!`. | Prefer (a). Markers stay if FAT `/hello` is present (smoke already requires it). |
| **H4 (rejected)** | Keep the embed as a dual path and only document the gap. | Allowed if (a) broke smoke. Not needed if FAT load stays green. |
| **H5 (chosen)** | Isolation: do **not** enable PAN (`pan: absent` on cortex-a57). Do **not** unmap identity `.data` / heap (SP and `GlobalAlloc` are still identity VAs). Print `ident: data-stay` / `ident: heap-stay`. Umbrella isolation stays Planned. | Advance the honest mile. A silent tear would fault. |
| **H6 (rejected)** | Switch `-cpu` so PAN appears, or yank `.data`/heap without relocating SP/allocator. | Forbidden by [ADR-026](ADR-026-pan-capability.md) / [ADR-025](ADR-025-identity-rodata-tear.md). |

## Decision

1. **Prior OS is `ba6541c`.** That is the A9 merge commit (`A9: Disconnect OS image from app payloads (ADR-030) (#60)`). No earlier `main` SHA has the slot/FAT path. Do not store a prior kernel ELF in git. `scripts/qemu-smoke.sh` builds that commit in a git worktree and QEMU `-kernel`s it with the **same** app bytes this tip published (`sha256sum` pin). Both boots must print `slot: fat` / `slot: mapped` / `slot: ok` and must not print `slot: embed` / `slot: probe missed`. This OS SHA must differ from `ba6541c`.
2. **No hello embed.** `src/libctos.rs` and `src/loader.rs` read FAT `/hello` through `fat::read_file` (same VFS `open` as A7). A2 still memcpy-flattens `PT_LOAD` at/after `EL0_PAGE` (not a guest ELF map). A3/A4 still parse and `ERET`. Size must match `HELLO_ELF_LEN` / `HELLO_LEN` from this `build.rs` so a swapped slot cannot silently satisfy those miles. Host smoke rejects `include_bytes!.*hello-libctos` under `src/`.
3. **Isolation honesty.** Serial `ident: data-stay` / `ident: heap-stay` after walking those identity pages. `#[test_case]` `identity_data_and_heap_still_mapped`. PAN enable stays **Planned** (`pan: id=0` / `pan: absent`; smoke still rejects `pan: enabled`). Do not claim “EL0 isolated.”
4. **Still Planned.** Identity `.data` / heap tear (needs SP relocate + high allocator VAs); PAN enable on `-cpu cortex-a57`; lower-EL IRQ while standing; EL0 entry without `TLBI VMALLE1`; umbrella isolation; A9 slot-disconnect **delta** vs the old embed; product “app hosting is done.”
5. **NFR-10 text** is revised in place (ID unchanged). Threat-model **v1.17**. Do not mint FR-16+ or NFR-15+.

## Honesty

Say “the same published `hello-libctos.elf` loaded on this OS and on `ba6541c`” only when host smoke printed both `cross-update … slot:ok` lines and the app `sha256` pin. Say “A2–A4 no longer embed the hello” only when `src/` has no hello `include_bytes!` **and** the FAT markers still pass. Say “identity `.data` / heap are still mapped” when `ident: data-stay` / `ident: heap-stay` print. Do **not** say:

- apps update independently / app hosting is done / OTA / A-B / containers
- EL0 isolated / PAN enabled / the kernel moved / identity mappings fully torn down
- a prior OS that does not exist in git
- Verified without this cloud (or GHA) qemu-smoke

## Consequences

- Code: `fat::read_file`, loader/libctos FAT path, teardown stay markers, `scripts/qemu-smoke.sh` embed + cross-update + stay greps.
- Docs: this ADR, [ADR-030](ADR-030-os-app-slots.md), A2–A4 ADRs (payload source), [ADR-025](ADR-025-identity-rodata-tear.md) / [ADR-026](ADR-026-pan-capability.md) (still Planned), ledger, Track A, threat-model v1.17.
