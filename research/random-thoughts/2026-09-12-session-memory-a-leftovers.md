# Session memory — 2026-09-12 (Track A leftovers)

Base `main` `ba6541c` (A9). Branch `cursor/track-a-leftovers-d2ee`. PR #64. Leftover ADR is **032** (#62 claimed 031 for Linux-compat).

Cross-update: `ba6541c` is the earliest main tip with the slot/FAT path. Built that SHA in a git worktree at `/tmp/ctos-prior-os-*` (not under `target/` — nested worktree applied `-Tlinker.ld` twice and overlapped `.ident_tear` with `.text`). Same published `hello-libctos.elf` (`sha256sum` pin) on both kernels.

GHA shallow clone failed closed (`missing prior OS commit`). Fix: `actions/checkout` `fetch-depth: 0` plus `git fetch --depth=1 origin $PRIOR_OS_SHA` if `cat-file` misses. Do not invent a kernel blob.

Embed-off: `fat::read_file("/hello")`. A2 flattens `PT_LOAD` at/after `EL0_PAGE`. A3/A4 parse the same bytes. Size must match `HELLO_ELF_LEN` / `HELLO_LEN` from this `build.rs`. Host smoke greps `include_bytes!.*hello-libctos` under `src/`.

Isolation: PAN ID still 0 on cortex-a57 — no `MSR PAN`. `.data`/heap stay; printed `ident: data-stay` / `ident: heap-stay`. Tear still needs SP relocate + high allocator VAs.

83 tests. qemu-smoke ok on `800f52d`. GitHub author of #64 is `artofdream`; merge hat is `cursor[bot]`.

Do not treat this file as the ledger.
