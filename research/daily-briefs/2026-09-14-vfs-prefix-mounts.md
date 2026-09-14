# 2026-09-14 — VFS prefix mounts (ADR-058)

- Product-depth train second mile after FAT16 delete.
- H1: static longest-prefix mount table — `/mem` + A6 probe names → memfs; `/` → FAT16.
- Kept `/hello` / `/probe` / write / readdir / delete / `slot: ok` / cross-update.
- Serial `vfs: mount` / `vfs: mounts`; `Running 103 tests`; agent-box `qemu-smoke: ok`.
- Not Linux `mount(2)`. CloudAgent HELD.
