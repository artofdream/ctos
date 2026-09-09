# Session memory — 2026-09-09 (M7)

QEMU virt RAM is `0x40000000`, default 128 MiB. `-kernel` TEXT_OFFSET `0x80000` → kernel `0x40080000`; DTB typically at RAM base (below the image). Do not parse FDT in M7 — linker `__kernel_end` + pinned `-m 128M`. 1 GiB L1 identity blocks (Device 0–1G, Normal RAM 1–2G) plus a 4 KiB window at `0x80000000` for map/unmap. Enable MMU only after `ensure_el1`. `spin::Mutex` cannot wrap the page tables (alignment). Do not start M8. Do not claim Raspberry Pi.
