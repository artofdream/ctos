# ADR-022 — Freestanding CRT / `libctos`

- Status: Accepted (CRT mile Verified when the serial / tests pass; Track A / app hosting still Planned)
- Date: 2026-09-12

## Context

Track A ([issue #31](https://github.com/artofdream/ctos/issues/31)) wants a freestanding program to *call* the A1 ABI without glibc or POSIX. [Issue #33](https://github.com/artofdream/ctos/issues/33) is this mile: a **documented, probed** CRT + `libctos` that wraps [ADR-021](ADR-021-svc-syscall-abi.md) numbers 16–18.

A1 already proved the kernel performs `exit` / `uart_write` / `yield` for a hand-encoded trampoline. That is not a library a program can link. A3 (ELF loader) must not start until something exists to link against.

Options:

1. **Wait for a loader.** Would block A2 on A3.
2. **C/asm static lib only.** Fine later; this tree is Rust-first and already uses `aarch64-ctos`.
3. **`no_std` crate + asm stubs + a hello payload copied onto today’s standing EL0 page.** Proves link + run without a guest ELF loader.

## Decision

1. **`libctos` crate** (`libctos/`). `no_std` wrappers `exit` / `uart_write` / `yield_now` over asm stubs (`sys.S`: `ctos_exit` / `ctos_uart_write` / `ctos_yield`). SVC immediates are 16 / 17 / 18. Reserved 0–2 stay ADR-013 probes — the CRT must not issue them.
2. **CRT** is `_start` in `libctos/src/crt0.S`: kernel `ERET` already set `SP_EL0`; `_start` calls `main`, then `ctos_exit`. No argv, no `environ`, no libc init. Not glibc/musl `crt0`.
3. **Hello payload** (`user/hello-libctos`). A `no_std` / `no_main` program links `libctos`, prints `libctos: hi` via `uart_write`, `yield`s, prints `libctos: ok`, returns 0. Linked at `paging::EL0_PAGE` (`0x80002000`) with `user/hello-libctos/linker.ld`.
4. **Not a guest loader.** `build.rs` compiles that program on the host and extracts `PT_LOAD` bytes at/after `EL0_PAGE` (size metadata + publish). The kernel copies a flatten of FAT `/hello` onto the standing EL0 page and `ERET`s — same style as A1. Host flatten in `build.rs` is not A3. Guest flatten of the FAT ELF ([ADR-031](ADR-031-track-a-leftovers.md)) is still A2 memcpy, not the A3 map.
5. **Code page + stack page.** The image must fit in 3584 bytes on `EL0_PAGE` (EL0-exec, no EL0 data — same AP as A1). Rust `main` saves `x30` on SP; that store cannot land on the code page (`SCTLR.WXN` forbids making it W+X). A second map-window page is EL0-RW NX. rust-lld’s extra ELF-header `PT_LOAD` at `0x80000000` is discarded at extract time.
6. **Fail-closed probe.** Hello serial `libctos: hi` + `libctos: ok` (from the payload via `uart_write`) + kernel `libctos: linked`. `scripts/qemu-smoke.sh` greps those and rejects `libctos: probe missed`. `#[test_case]` covers the trip and that the blob encodes SVC #16/#17/#18 and not #0/#1/#2. Existing `svc:` / `el0:` markers stay.
7. **Honesty.** Say “a program linked against libctos issued the A1 ABI and the kernel performed the documented effect” only when the serial / tests pass. Do **not** say: app hosting is done, Linux ABI, POSIX, userspace, “EL0 isolated,” or “secure OS.” Track A A3–A9 stay Planned.
8. **NFR-10 text** is revised in place (ID unchanged) to name this CRT mile. Do not mint FR-16+ or NFR-15+. Guest ELF load is [ADR-023](ADR-023-elf-pt-load-loader.md), not this ADR.

## Consequences

- Code: `libctos/`, `user/hello-libctos/`, `build.rs`, `src/libctos.rs`.
- Docs: [syscall.md](../framework/syscall.md), [el0.md](../framework/el0.md), [building-or-porting.md](../framework/building-or-porting.md). Roadmap cites #33 / Track A #31.
- A3 (ELF / raw loader into user TTBR0) may load a *separate* image. It is not this PR.
