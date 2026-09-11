# Building or porting

Honesty first: there is **no** userspace ABI to compile against, and no libc. ctos is a freestanding `no_std` kernel on a custom target. Status words need a probe in the [honesty ledger](honesty-ledger.md).

What already runs: [apps-today.md](apps-today.md). KPIs and trade-offs: [overview.md](overview.md).

A docs website at https://ctos.artof.link is **Planned**. A Route 53 CNAME exists; this tree does not publish Pages. Do not claim that URL works.

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
- filesystem, signals, sockets, `environ`, TLS as Linux defines them
- a C runtime (`crt0`, libgcc helpers as a POSIX process)

Do not publish a “port busybox / musl to ctos” guide that skips those gaps. That work would be many ADRs, not a weekend `#ifdef`. Frozen Out list: [fr-nfr.md](../02-requirements/fr-nfr.md) (userspace processes, POSIX, networking).

## Later Planned — SVC ABI + `libctos` (freestanding EL0)

Standing EL0 is a **dual-SVC stub** (`SVC #1` stay / `SVC #2` restore) plus first-mile `SVC #0`. It is not a syscall table and not a libc.

**Planned** (not in tree, not Verified):

1. Isolation miles first: PAN (usually absent on `cortex-a57`), identity `.rodata` / `.data` / heap tear, umbrella EL0 isolation ([el0.md](el0.md), [ADR-013](../03-adr/ADR-013-el0-isolation-direction.md)).
2. A **stable SVC ABI** written as a later ADR (numbers, registers, error model). Do not silently grow `#1` / `#2` into POSIX.
3. A freestanding **`libctos`** (name reserved here as intent only — no crate today) that a future EL0 program could link against `no_std`, talking that ABI. Still not glibc. Still not `exec` of a Linux ELF.

Until those probes exist, “write a user program for ctos” is **Planned**. The easiest thing you can do today remains an in-tree EL1 task.

## Do not invent

- A porting guide that assumes POSIX, a shell, Python, or containers
- A claim that https://ctos.artof.link is a live docs site
- “Secure OS,” “the kernel moved,” or “EL0 isolated”

Cite the ledger for any Verified SHA you quote.
