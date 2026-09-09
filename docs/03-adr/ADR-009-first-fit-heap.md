# ADR-009 — First-fit heap on identity-mapped frames

- Status: Accepted
- Date: 2026-09-09

## Context

[FR-10](../02-requirements/fr-nfr.md) requires a `GlobalAlloc` heap so `alloc` types (`Box`, `Vec`) work in the kernel. Roadmap M8 is that path. The scheduler is M9.

M7 already identity-maps virt RAM and hands out 4 KiB frames from `__kernel_end` through 128 MiB ([ADR-008](ADR-008-identity-map-frame-allocator.md)). Frames are physical pages, not a byte allocator.

Options:

1. **Bump-only heap** (no `dealloc`). `Box`/`Vec` construct, but drop leaks. Reuse is unproved.
2. **`linked_list_allocator` crate.** Common rust-osdev choice; extra dependency for one milestone.
3. **First-fit free list on a fixed contiguous frame run** taken from the bump pool after the MMU is on.
4. **Grow-on-demand** via the M7 map window (`0x8000_0000`). Extra paging surface; not needed to prove FR-10.
5. **Buddy / slab.** Overkill for a Box/Vec smoke.

## Decision

1. **Fixed 64 KiB heap** (16 frames) via `frame::alloc_contiguous` after `paging::init`. Identity VA == PA. Do not map through the M7 probe window.
2. **First-fit + address-sorted coalesce** in `src/heap.rs`. `dealloc` is real; the hello probe requires a freed `Box` pointer to be reused.
3. **`#[global_allocator]` + `extern crate alloc`.** `.cargo/config.toml` `build-std` includes `alloc`. OOM returns a null pointer; `#[alloc_error_handler]` prints `heap: oom` on the raw UART and parks (or `SYS_EXIT` 1 under test). The handler must not format — formatting an OOM can allocate again.
4. **Serial marker `heap: ok`.** Fail closed on `heap: probe missed`. `#[test_case]` covers pool bounds, `Box` reuse, and `Vec` growth.
5. **Not a growing heap, not userspace, not the scheduler.** A later slab / grow-on-demand / higher-half heap needs a new ADR.

## Consequences

- `scripts/qemu-smoke.sh` requires `heap: ok` after `paging: ok`, then still requires M2–M7 strings.
- The frame pool shrinks by 64 KiB at boot. M7 frame/map tests still have the rest of the 128 MiB guest.
- This ADR does not claim Raspberry Pi or a production allocator. M9 task stacks on this heap are [ADR-010](ADR-010-cooperative-rr-el1.md).
