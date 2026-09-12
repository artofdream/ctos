# Daily brief — 2026-09-12 (Track A / A3 guest ELF loader)

## Where we stopped

Draft PR https://github.com/artofdream/ctos/pull/53 (`cursor/a3-elf-loader-ttbr0-49f0`). Parent is `main` `61b2640` (A2 / ADR-022). One PR: ADR-023 guest ELF64 `PT_LOAD` into user TTBR0. No new FR/NFR IDs. App hosting unclaimed. A4–A9 stay **Planned**.

Cloud `scripts/qemu-smoke.sh` **Verified** (2026-09-12, QEMU 8.2.2, `rustc` 1.100.0-nightly `0fc141305`): host `perf: elf-size bytes=4101056`; `loader: mapped segs=3 entry=0x80002000` / `loader: ok`; payload `libctos: hi` / `libctos: ok` on the loaded trip; A1 `svc:*` and A2 `libctos: linked` still present; `Running 64 tests` all `[ok]`; force-fail exit 1.

## Do next

1. Human or MRC review. Author does not merge (ADR-002).
2. A4: standing EL0 as **normal** mode (not only a bounded probe that enters and leaves). Keep the A3 loader as the way a payload appears.
3. Do not claim Linux ELF ABI, `PT_INTERP`, glibc, or “app hosting is done.”

## Honesty

- Cloud Verified is this QEMU virt guest on this revision.
- Image is still embedded (`include_bytes!`). No VFS.
