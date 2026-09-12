# ADR-023 — Guest ELF64 PT_LOAD loader into user TTBR0

- Status: Accepted (loader mile Verified when the serial / tests pass; Track A / app hosting still Planned)
- Date: 2026-09-12

## Context

Track A ([issue #31](https://github.com/artofdream/ctos/issues/31)) wants a freestanding image loaded into **user** maps and entered with `ERET`. [Issue #34](https://github.com/artofdream/ctos/issues/34) is this mile.

[ADR-022](ADR-022-libctos-crt.md) already links `user/hello-libctos` against `libctos`. That hello is **host-built**. `build.rs` extracts `PT_LOAD` and the kernel **memcpy**s the blob onto `paging::EL0_PAGE`. That is not a guest loader.

A3 must parse and map on the **guest**. The bytes may still be embedded (no VFS yet — A6/A7). The point is map + load + `ERET`, not host flatten-and-copy onto a fixed probe page alone.

Existing A1 `svc:*` and A2 `libctos:*` markers stay. A4–A9 stay out of this PR.

## Hypotheses

| ID | Option | Notes |
| --- | --- | --- |
| **H1 (chosen)** | Minimal **ELF64 LE AArch64 `ET_EXEC`**, `PT_LOAD` only | The in-tree payload is already that ELF. A2 already walks `PT_LOAD` on the host. Guest parse is the missing mile. |
| **H2 (rejected)** | Custom raw image (base VA + entry + blob) | Smaller parser, but the host would flatten it — same shape as A2. Would not prove guest ELF walk. |
| **H3 (rejected)** | Linux ELF ABI / `PT_INTERP` / `ET_DYN` / TLS / glibc | Inflates the claim. Fail-closed: reject `PT_INTERP`. Not Track B. |

## Decision

1. **Guest parser** in `src/loader.rs`. Accept ELF64 little-endian AArch64 `ET_EXEC`. Walk program headers. Load `PT_LOAD` only. Ignore `PT_PHDR` / `PT_GNU_STACK` / notes. **Reject `PT_INTERP`.** Reject a `PT_LOAD` that is W+X (segment flags or two segments that union to W+X on one page). `SCTLR.WXN` is already on ([ADR-015](ADR-015-ro-nx-text-data.md)); do not map a writable executable leaf.
2. **User TTBR0 maps.** Each page of a `PT_LOAD` is a freshly allocated frame mapped in the shared map-window L3 (`paging::MAP_WINDOW` … `+ MAP_WINDOW_SIZE`). That L3 is already in `L1_USER`, so EL0 and EL1 both see it. Permissions: `PF_X` → `map_el0_exec` (EL0 fetch, no EL0 data; EL1 may write the identity PA before / while mapping); `PF_W` → `map_el0_rw`; R-only → `map_el0_ro`. Page-granular **union**: `.text` RX and `.rodata` R on one 4 KiB page become Exec. Not a new user address space.
3. **Copy + BSS.** File bytes are written to the frame’s identity PA. `p_memsz > p_filesz` is zeroed. I-cache is synced on executable pages. `e_entry` must land in an Exec page.
4. **Stack.** A separate EL0-RW NX page at `paging::LOADER_STACK_VA` (`0x80007000`), not the A2 code page. Rust `main` still saves `x30` on `SP_EL0`.
5. **Embedded image.** `build.rs` still emits the A2 flattened `.bin` **and** the raw ELF (`hello-libctos.elf`). The kernel `include_bytes!` the ELF. No filesystem. Not a second QEMU `-kernel`.
6. **ERET.** `el0::install_standing(e_entry, stack_top, user_ttbr0)` then `exception::eret_to_el0`. `SYS_EXIT` returns to EL1 as today. A2’s memcpy path stays so `libctos:*` markers remain.
7. **Fail-closed probe.** Hello serial `loader: mapped` + `loader: ok` after a successful trip. The loaded payload still prints `libctos: hi` / `libctos: ok` via `uart_write`. `scripts/qemu-smoke.sh` greps those and rejects `loader: probe missed`. `#[test_case]` covers parse, `PT_INTERP` reject, W+X reject (flags and overlapping pages), RX+R union on the hello page, and the full map + `ERET` trip. Existing `svc:` / `libctos:` / `el0:` markers stay.
8. **Honesty.** Say “the guest parsed a freestanding ELF64, mapped `PT_LOAD` into user TTBR0, and `ERET`ed to `e_entry`” only when the serial / tests pass. Do **not** say: Linux ELF ABI, `PT_INTERP`, glibc, app hosting is done, POSIX, userspace, “EL0 isolated,” or “secure OS.” Standing-as-normal is [ADR-024](ADR-024-standing-el0-normal.md). Track A A5–A9 stay Planned.
9. **NFR-10 text** is revised in place (ID unchanged) to name this loader mile. Do not mint FR-16+ or NFR-15+.

## Consequences

- Code: `src/loader.rs`, `paging::map_el0_ro` / `window_range_ok` / `LOADER_STACK_VA`, `build.rs` emits the ELF, `scripts/qemu-smoke.sh`.
- Docs: [el0.md](../framework/el0.md), [syscall.md](../framework/syscall.md), [building-or-porting.md](../framework/building-or-porting.md). Roadmap cites #34 / Track A #31.
- A4 (standing EL0 as **normal** mode) keeps this loader as the way a payload appears ([ADR-024](ADR-024-standing-el0-normal.md)).
