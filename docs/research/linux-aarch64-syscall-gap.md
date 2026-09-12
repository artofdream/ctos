# B2 — Linux AArch64 syscalls vs ctos SVC

Track B child [issue #42](https://github.com/artofdream/ctos/issues/42) (parent [#40](https://github.com/artofdream/ctos/issues/40)). Frame: [ADR-031](../03-adr/ADR-031-linux-compat-goals.md). Freestanding ABI: [ADR-021](../03-adr/ADR-021-svc-syscall-abi.md) / [syscall.md](../framework/syscall.md).

This page is a **gap map**. It does **not** add Linux syscall numbers to `src/`. It does **not** decide compat layer vs reimplement vs never ([B6 / #46](https://github.com/artofdream/ctos/issues/46)). **Not claiming Linux userspace.** File presence of this note is not a Linux ABI.

## Probe (what Verified means here)

| Claim | Probe | Status |
| --- | --- | --- |
| Linux AArch64 numbers in the table | Read `include/uapi/asm-generic/unistd.h` from Linux **v6.10** (`torvalds/linux`). AArch64 uses that generic table (`svc #0`, number in `x8`). | Document inspection |
| ctos public / reserved SVC surface | Read `src/syscall.rs` (`SYS_*` 16–23) + [ADR-021](../03-adr/ADR-021-svc-syscall-abi.md) + [ADR-027](../03-adr/ADR-027-thin-vfs-memfs.md) + reserved 0–2 in [ADR-013](../03-adr/ADR-013-el0-isolation-direction.md) on this tree | Document inspection |
| Linux userspace / musl / Alpine on the guest | No such serial marker | **Not claimed.** Ledger “Guest runs host apps” stays **Planned**. |

A later kernel `qemu-smoke` is a different row. This mile did not edit `src/`.

## Status words

| Status | Means |
| --- | --- |
| **present** | Same Linux AArch64 number **and** comparable semantics, callable by a Linux `svc #0` / `x8` binary with no translation. |
| **partial** | A Track A SVC exists with a related effect. Number, calling convention, args, or meaning still differ. |
| **absent** | No ctos analog. B3–B6 may still discuss a *named* subset. Not a promise to implement. |
| **never-per-ADR-031** | Would imply a full Linux host / distro / container promise that [ADR-031](../03-adr/ADR-031-linux-compat-goals.md) already ruled out (guest OCI also [ADR-029](../03-adr/ADR-029-containers-nongoal.md)). |

**No Linux AArch64 syscall is `present`.** The convention and the number space both miss.

## Calling convention (the first gap)

| Item | Linux AArch64 | ctos (ADR-021) |
| --- | --- | --- |
| Instruction | `svc #0` (immediate is always 0) | `SVC #<n>` — `n` **is** the number |
| Number register | `x8` | Immediate (not `x8`) |
| Args | `x0`–`x5` | `x0`, `x1`, `x2` only |
| Return | `x0`; negative Linux errno | `x0`; `0` or `u64::MAX` reject (not errno) |
| Unknown number | `-ENOSYS` | Unhandled immediate **parks** (fail-closed) |

Linux `svc #0` **collides** with the ADR-013 first-mile probe (`SVC #0`). [ADR-021](../03-adr/ADR-021-svc-syscall-abi.md) rejected adopting the Linux convention for that reason. A translation shim would be a later epic after B6 — not this page.

Public ctos numbers **16–23** are also **not** the Linux meaning of those integers:

| Immediate / `x8` | Linux AArch64 (v6.10 generic) | ctos |
| --- | --- | --- |
| 0 | `io_setup` | ADR-013 first-mile return (reserved) |
| 1 | `io_destroy` | ADR-013 standing announce (reserved) |
| 2 | `io_cancel` | ADR-013 standing restore (reserved) |
| 16 | `fremovexattr` | `exit` |
| 17 | `getcwd` | `uart_write` |
| 18 | `lookup_dcookie` | `yield` |
| 19 | `eventfd2` | `fs_create` |
| 20 | `epoll_create1` | `fs_open` |
| 21 | `epoll_ctl` | `fs_read` |
| 22 | `epoll_pwait` | `fs_write` |
| 23 | `dup` | `fs_close` |

You cannot “just accept Linux numbers” in the SVC immediate, and you cannot treat today’s 16–23 as Linux.

## ctos surface (inspection)

Reserved **0–2** are probe-only. Public:

| ctos # | Name | Related Linux name(s) | Why not `present` |
| --- | --- | --- | --- |
| 16 | `exit` | `exit` (93), `exit_group` (94) | Ends the standing EL0 trip; no process table; not `exit_group`. |
| 17 | `uart_write` | `write` (64) to a TTY | PL011 only; no fd; cap 64; `\n` → `\r\n`. |
| 18 | `yield` | `sched_yield` (124) | Dispatch-only hint; does **not** run EL1 `sched::yield_now` ([ADR-010](../03-adr/ADR-010-cooperative-rr-el1.md)). |
| 19 | `fs_create` | `openat` (56) + `O_CREAT` | Flat memfs name; no `dirfd`, flags, or mode. |
| 20 | `fs_open` | `openat` (56) | Existing name only; FAT `/probe` and `/hello` are the known on-disk files. |
| 21 | `fs_read` | `read` (63) | Handle 1–4; cap 64; no stdin. UART RX is kernel-side, not this SVC. |
| 22 | `fs_write` | `write` (64) | memfs only (FAT write is `ReadOnly`); cap 64; not fd 1/2. |
| 23 | `fs_close` | `close` (57) | Drops the handle; the file stays. Four handles max. |

Unknown immediates park. Not POSIX. Not Linux VFS.

## Gap table (representative Linux AArch64 set)

Numbers from Linux **v6.10** `asm-generic/unistd.h`. AArch64 has **no** `fork`, `open`, `creat`, `dup2`, `pipe`, `poll`, or `select` syscalls — those are `clone` / `openat` / `dup3` / `pipe2` / `ppoll` / `pselect6`. `__NR_syscalls` in that header is **463**. This table is a **slice**, not the full list.

### Process and lifetime (B3)

| Linux # | Name | ctos status | Notes |
| --- | --- | --- | --- |
| 93 | `exit` | **partial** | Analog `SYS_EXIT` **16**. Halts the EL0 trip and returns to the EL1 caller. Status is recorded. Not a thread/process exit. |
| 94 | `exit_group` | **absent** | No process group. musl/glibc CRT usually ends here. |
| — | `fork` | **absent** | **Not in the Linux AArch64 table.** Userspace `fork()` is `clone`. Stance: [ADR-035](../03-adr/ADR-035-process-model-standing-el0.md) / [B3 / #43](https://github.com/artofdream/ctos/issues/43). |
| 220 | `clone` | **absent** | No child address space, no `CLONE_*`. Standing EL0 ([ADR-024](../03-adr/ADR-024-standing-el0-normal.md)) is one loaded trip, not `clone` ([ADR-035](../03-adr/ADR-035-process-model-standing-el0.md)). `CLONE_NEW*` is **never-per-ADR-031** (see namespace rows). |
| 435 | `clone3` | **absent** | Same class as `clone`. |
| 221 | `execve` | **absent** | A3 maps freestanding `PT_LOAD` and **rejects `PT_INTERP`**. A9 FAT `/hello` is an EL1 load path, not `execve` ([ADR-035](../03-adr/ADR-035-process-model-standing-el0.md)). ELF / auxv / interpreter gap: [ADR-033](../03-adr/ADR-033-linux-elf-auxv-pt-interp.md) (B4). |
| 281 | `execveat` | **absent** | Same class as `execve`. |
| 260 | `wait4` | **absent** | No child to wait for. [ADR-035](../03-adr/ADR-035-process-model-standing-el0.md). |
| 95 | `waitid` | **absent** | [ADR-035](../03-adr/ADR-035-process-model-standing-el0.md). |
| 129 | `kill` | **absent** | No pid / signal delivery. |
| 172 | `getpid` | **absent** | No process table. |
| 178 | `gettid` | **absent** | No tid. |
| 96 | `set_tid_address` | **absent** | Typical musl/glibc `_start` requirement. |

### File descriptors and I/O (B5 concept compare: [ADR-034](../03-adr/ADR-034-linux-vfs-vs-thin-ctos.md))

| Linux # | Name | ctos status | Notes |
| --- | --- | --- | --- |
| 56 | `openat` | **partial** | `fs_open` **20** / `fs_create` **19**. Path is `/` + `[a-z0-9_-]` (1–15). No `dirfd`, `O_*`, or mode. At most 8 files. [ADR-027](../03-adr/ADR-027-thin-vfs-memfs.md). |
| 437 | `openat2` | **absent** | No `open_how`. |
| 57 | `close` | **partial** | `fs_close` **23**. File stays after close. |
| 63 | `read` | **partial** | `fs_read` **21**. Cap 64. No fd 0 / line discipline. |
| 64 | `write` | **partial** | `uart_write` **17** (console, no fd) **or** `fs_write` **22** (memfs). Neither is POSIX `write(1, …)`. |
| 65 / 66 | `readv` / `writev` | **absent** | No iovec. |
| 67 | `pread64` | **absent** | Handle offset exists inside VFS; no positioned SVC. |
| 62 | `lseek` | **absent** | No seek SVC. |
| 29 | `ioctl` | **absent** | No TTY (`TCGETS` / `TIOCGWINSZ`), no block `ioctl`. PL011 is kernel-owned. |
| 25 | `fcntl` | **absent** | No `F_GETFL` / `F_DUPFD`. |
| 23 / 24 | `dup` / `dup3` | **absent** | Handles 1–4 only; no dup. Linux 23 is `dup`; ctos 23 is `fs_close`. |
| 59 | `pipe2` | **absent** | No pipes. |
| 61 | `getdents64` | **absent** | No directory listing. Flat namespace. |
| 79 / 80 | `newfstatat` / `fstat` | **absent** | No `stat`. |
| 291 | `statx` | **absent** | No `stat`. |
| 48 | `faccessat` | **absent** | |
| 34 | `mkdirat` | **absent** | No directories. |
| 35 | `unlinkat` | **absent** | |
| 38 | `renameat` | **absent** | |
| 49 | `chdir` | **absent** | No cwd. |
| 17 | `getcwd` | **absent** | Linux 17 is `getcwd`; ctos 17 is `uart_write`. |

### Memory (loader / libc heap)

| Linux # | Name | ctos status | Notes |
| --- | --- | --- | --- |
| 214 | `brk` | **absent** | Kernel first-fit heap is EL1 ([ADR-009](../03-adr/ADR-009-first-fit-heap.md)). No user program break. |
| 222 | `mmap` | **absent** | A3 maps `PT_LOAD` at load time into the user window. No mmap SVC. Anonymous / file maps are out. |
| 215 | `munmap` | **absent** | |
| 226 | `mprotect` | **absent** | Image W^X is a kernel policy ([ADR-015](../03-adr/ADR-015-ro-nx-text-data.md)), not an EL0 syscall. |
| 233 | `madvise` | **absent** | |

### Scheduling, time, sync, identity

| Linux # | Name | ctos status | Notes |
| --- | --- | --- | --- |
| 124 | `sched_yield` | **partial** | Analog `yield` **18** is a recorded hint and `ERET` to EL0. It does not switch EL1 tasks. |
| 101 | `nanosleep` | **absent** | CNTP tick is EL1. |
| 113 | `clock_gettime` | **absent** | No guest clock SVC. CNTPCT probes are kernel/NFR-07. |
| 98 | `futex` | **absent** | No user futex. |
| 99 | `set_robust_list` | **absent** | Typical libc `_start`. |
| 134 / 135 / 139 | `rt_sigaction` / `rt_sigprocmask` / `rt_sigreturn` | **absent** | Lower-EL IRQ still parks. No POSIX signals. |
| 73 | `ppoll` | **absent** | |
| 160 | `uname` | **absent** | |
| 167 | `prctl` | **absent** | |
| 163 / 261 | `getrlimit` / `prlimit64` | **absent** | |
| 278 | `getrandom` | **absent** | |

### Sockets (not a Track B promise)

| Linux # | Name | ctos status | Notes |
| --- | --- | --- | --- |
| 198 | `socket` | **absent** | No virtio-net, no stack. Same class as the host-apps “sockets” gap. Not a B2 implementation item. |

### Namespace / mount — never on this track

| Linux # | Name | ctos status | Notes |
| --- | --- | --- | --- |
| 97 | `unshare` | **never-per-ADR-031** | Linux namespaces. Guest OCI is a **non-goal** ([ADR-029](../03-adr/ADR-029-containers-nongoal.md)). |
| 268 | `setns` | **never-per-ADR-031** | Same. |
| 40 | `mount` | **never-per-ADR-031** | Linux mount/overlay class. A7 FAT is not `mount(2)`. Concept compare: [ADR-034](../03-adr/ADR-034-linux-vfs-vs-thin-ctos.md) (B5). Must not grow POSIX mount. |
| 39 | `umount2` | **never-per-ADR-031** | Same. |
| 41 | `pivot_root` | **never-per-ADR-031** | Same. |

`clone` with `CLONE_NEWNS` / `CLONE_NEWPID` / `CLONE_NEWNET` is the same **never-per-ADR-031** class. A later `clone` subset, if B6 ever names one, would still exclude those flags.

## If a subset is ever useful (not a decision)

Issue #42 asked to prioritize a minimal subset **if any**. This is a research hint for B6, not an implementation list.

1. **In-tree `libctos` apps need no Linux numbers.** A1–A9 already have `exit` / `uart_write` / `yield` / memfs / FAT `/hello`. Keep that path.
2. **A static musl “hello” is not a small subset.** Besides convention translation, typical CRT wants `set_tid_address`, `exit_group`, `brk` or `mmap`, often `write` on fd 1, `uname` / `geteuid`-class probes, and signal/`prctl` stubs. That is already a libc-shaped surface. ADR-031 rejected “runs musl / Alpine” as a Track B *promise*.
3. **The smallest *named* research slice** (only if B6 chooses a compat path, and only with a probe) would still be: **convention translation** + `exit`/`exit_group` + `write` to a console fd. `openat`/`read`/`close` are the next file slice. `brk`/`mmap` are the next heap slice. `clone`/`execve`/`wait4` are the process gap ([ADR-035](../03-adr/ADR-035-process-model-standing-el0.md)); they are not a small add-on.
4. **Do not start with ioctl, sockets, or mount.** ioctl is a device encyclopedia. Sockets need a stack. mount/unshare stay **never-per-ADR-031**.

B3 is **Documented** ([ADR-035](../03-adr/ADR-035-process-model-standing-el0.md)). B4 is **Documented** ([ADR-033](../03-adr/ADR-033-linux-elf-auxv-pt-interp.md)). B5 is **Documented** ([ADR-034](../03-adr/ADR-034-linux-vfs-vs-thin-ctos.md)). They must not add Linux numbers to `src/`. B6 may choose **never**.

## Honesty

- Analysis **Verified** means: this file was read against `src/syscall.rs` and Linux v6.10 `unistd.h`. It is **not** ABI Verified and **not** Linux userspace.
- Do not say “ctos implements `write(2)`” because `uart_write` or `fs_write` exists.
- Do not mint FR-16+ / NFR-15+. Frozen set: [fr-nfr.md](../02-requirements/fr-nfr.md).
- Guest containers stay **non-goal**. Host `docker-smoke.sh` is unrelated.

Roadmap: [track-b.md](../04-roadmap/track-b.md). Extra stance: [host-apps.md](../framework/host-apps.md).
