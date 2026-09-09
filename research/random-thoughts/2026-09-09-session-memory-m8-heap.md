# Session memory — 2026-09-09 (M8)

First-fit free list on 16 contiguous bump frames after `paging::init` (identity VA==PA). Do not use the M7 map window for the heap. `dealloc` must be real — hello probe requires Box pointer reuse. `#[alloc_error_handler]` must not format (OOM recurse). `build-std` needs `alloc`. Do not start M9. Do not claim Raspberry Pi.
