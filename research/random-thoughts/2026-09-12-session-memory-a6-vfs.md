# Session memory — 2026-09-12 (Track A / A6)

Implemented thin VFS + memfs on `cursor/a6-thin-vfs-memfs-766a` from `main` `99fa9ff`.

- `src/vfs.rs`: `VfsOps` + `MemFs` (8 files × 256 B, 4 handles, flat `/[a-z0-9_-]+` paths, heap `Vec<u8>`).
- SVC 19–23 in `src/syscall.rs`; `libctos` wrappers. Hello payload still 16–18 only (A2–A4 markers unchanged).
- Kernel `/kprobe` + EL0 trampoline `/eprobe` (`memfs-el0`). Smoke greps `fs: create/write/read/el0/ok`.
- First `qemu-smoke` needed `apt-get install qemu-system-arm` on this VM. Then exit 0: 73 tests, A1–A5 strings still present.
- Did not implement A7–A9. Did not claim FAT or POSIX.

Do not treat this file as the honesty ledger.
