# Building or porting

Honesty first: there is a tiny `libctos` CRT for the A1 SVC ABI, a guest `PT_LOAD` loader for that in-tree ELF, and **no** libc. ctos is a freestanding `no_std` kernel on a custom target. Status words need a probe in the [honesty ledger](honesty-ledger.md).

What already runs: [apps-today.md](apps-today.md). Recipes: [What can run today](../overview/what-can-run.md), in-tree `user/README.md`. KPIs and trade-offs: [overview.md](overview.md).

Site source of truth: [Building or porting](../overview/porting.md). This page is extra stance. HTTPS at https://ctos.artof.link is **Verified** (2026-09-11 after #30).

## Easiest path — in-tree `no_std` cooperative EL1

The only path that matches today’s probes is **another kernel task in this repo**, built for `aarch64-ctos.json`, proven with the same smoke you already have.

### Target and build

| Piece | What it is |
| --- | --- |
| `aarch64-ctos.json` | Custom rustc target: `os: none`, abort, soft-float, static, `rust-lld`. Not `aarch64-unknown-linux-gnu`. |
| `.cargo/config.toml` | Pins that JSON, `build-std` (`core` / `alloc` / `compiler_builtins`), `-Tlinker.ld`, QEMU runner. |
| `rust-toolchain.toml` | Nightly + `rust-src` / `llvm-tools-preview`. |

```bash
cargo build                 # ELF at target/aarch64-ctos/debug/ctos
cargo run                   # qemu-system-aarch64 -machine virt
./scripts/qemu-smoke.sh     # hello greps + cargo test + force-fail
./scripts/docker-smoke.sh   # same smoke in linux/arm64 (do not pin amd64)
```

A machine that has not run `qemu-smoke.sh` (or Docker / GHA equivalent) has **Unknown** boot. Idle tip: sponsor Docker is **Verified** on `main` `e80dc93` (50 tests, `ident: reloc n=12`, live pages=37, force-fail). That is one host, not this VM and not a Pi.

### Add a cooperative EL1 task

Same shape as [apps-today.md](apps-today.md) (`src/sched.rs` `task_a` / `task_b`):

1. Write a `fn my_task()` that uses `alloc` / UART / `sched::yield_now()` only. No `std`, no files, no sockets.
2. `spawn(my_task)` next to the existing workers (heap must be up; stacks are heap `Vec`s).
3. Print a **new** serial marker (`sched: beat n=…` or similar). Teach `scripts/qemu-smoke.sh` to grep it. Fail closed on miss.
4. One milestone → one branch → one GitHub PR. Author ≠ merger ([ADR-002](../03-adr/ADR-002-pr-identity-split.md)).

A heartbeat / counter loop is the natural variant. It is **not** in the tree until that PR lands. Do not claim it is Verified from this paragraph.

Stay on AArch64 QEMU `virt` / PL011 ([ADR-003](../03-adr/ADR-003-primary-isa-aarch64.md)). Do not restore `bootloader` 0.9 or VGA as primary.

## POSIX / glibc port — not easy, not started

A Linux, musl, or glibc binary will **not** run. Missing, among other things:

- `exec` / ELF loader / dynamic linker
- syscall table (`read` / `write` / `open` / `mmap` / `clone` / …)
- filesystem as Linux defines it (thin VFS + memfs + read-only FAT16 is not POSIX `open` — [filesystem.md](filesystem.md)), signals, sockets, `environ`, TLS as Linux defines them
- a C runtime (`crt0`, libgcc helpers as a POSIX process)

Do not publish a “port busybox / musl to ctos” guide that skips those gaps. That work would be many ADRs, not a weekend `#ifdef`. Frozen Out list: [fr-nfr.md](../02-requirements/fr-nfr.md) (userspace processes, POSIX, networking).

## `libctos` (A2) — freestanding EL0 CRT

Standing EL0 is still a **dual-SVC stub** (`SVC #1` stay / `SVC #2` restore) plus first-mile `SVC #0`, plus the A1 public ABI trip. A2 adds a `no_std` crate you can link:

| Piece | What it is |
| --- | --- |
| `libctos/` | Wrappers `exit` / `uart_write` / `yield_now` over `SVC #16` / `#17` / `#18`. Must not issue `#0`–`#2`. |
| `libctos/src/crt0.S` | `_start` → `main` → `ctos_exit`. No argv / environ. |
| `user/hello-libctos/` | Hello that prints `libctos: hi` / `libctos: ok` via `uart_write`. Recipe: `user/hello-libctos/README.md`. |

That is **not** glibc. **Not** `exec` of a Linux ELF. The hello image is host-built. A2 copies the flattened `PT_LOAD` onto the standing EL0 page ([ADR-022](../03-adr/ADR-022-libctos-crt.md)). A3 parses the **same ELF** on the guest and maps `PT_LOAD` into user TTBR0 ([ADR-023](../03-adr/ADR-023-elf-pt-load-loader.md)). File presence is not the probe — see the ledger for `libctos: ok` and `loader: ok`.

## Guest loader (A3)

`src/loader.rs` walks ELF64 LE AArch64 `ET_EXEC` program headers, maps `PT_LOAD` pages in the user map-window, and `ERET`s to `e_entry`. Rejects `PT_INTERP` and W+X. A2–A4 still `include_bytes!`. A9 also reads FAT `/hello` ([ADR-030](../03-adr/ADR-030-os-app-slots.md)). Not a Linux ABI.

## Standing EL0 as normal mode (A4)

A4 keeps the A3 loader as the way a payload appears. `install_task` + `ERET` is the supported path: the loaded image runs until `exit`; `is_active()` is true only for that lifetime; an unexpected EL0 fault restores fail-closed ([ADR-024](../03-adr/ADR-024-standing-el0-normal.md)). Serial `el0: task-ok`. File presence is not that probe.

That is **not** a process, not POSIX, and not “EL0 isolated.”

**Still Planned:**

1. Isolation miles: PAN **enable** (absent on `cortex-a57` — [ADR-026](../03-adr/ADR-026-pan-capability.md)), identity `.data` / heap tear (`.rodata` is [ADR-025](../03-adr/ADR-025-identity-rodata-tear.md)), umbrella EL0 isolation ([el0.md](el0.md), [ADR-013](../03-adr/ADR-013-el0-isolation-direction.md)). A5 took the `.rodata` + PAN ID-field cut.
2. A9 cross-update (same app on OS n and n+1). The first cut is two artifacts + FAT `/hello` ([ADR-030](../03-adr/ADR-030-os-app-slots.md)). A2–A4 still embed.

“Write a user program for ctos” still means: link `libctos` in-tree, publish `target/hello-libctos.elf`, and put it on FAT `/hello` — **or** add an EL1 task. The easiest thing you can do today remains an in-tree EL1 task.

The **OS image vs app payload** split ([A9 #48](https://github.com/artofdream/ctos/issues/48)) has a first cut. Cross-update is not Verified. See [overview.md](overview.md) and [immutability.md](immutability.md).

## Do not invent

- A porting guide that assumes POSIX, a shell, Python, or containers ([host-apps.md](host-apps.md): containers are a **non-goal**)
- A claim about the docs URL that skips the [ledger](honesty-ledger.md) (HTTPS is Verified as of 2026-09-11; do not invent extra site KPIs)
- “Secure OS,” “the kernel moved,” or “EL0 isolated”

Cite the ledger for any Verified SHA you quote.
