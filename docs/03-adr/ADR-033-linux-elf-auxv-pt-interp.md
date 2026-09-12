# ADR-033 — Linux ELF / auxv / PT_INTERP vs the freestanding loader

- Status: Accepted (gap ADR; no Linux ELF / dynamic-linker claim)
- Date: 2026-09-12

## Context

Track B child [B4 / #44](https://github.com/artofdream/ctos/issues/44) (parent [#40](https://github.com/artofdream/ctos/issues/40)) asks what would be needed to load a **musl-linked** binary versus today’s freestanding image, and to **keep the Track A loader reusable**.

Frame: [ADR-031](ADR-031-linux-compat-goals.md). That frame says B4 must **not accept `PT_INTERP`**, must not claim Linux userspace, and must not replace the freestanding Track A path. Syscall convention is already a gap ([linux-aarch64-syscall-gap.md](../research/linux-aarch64-syscall-gap.md) / B2). Process `execve` is [B3 / #43](https://github.com/artofdream/ctos/issues/43). This ADR does **not** write B3, B5, or B6.

Track A A3 ([ADR-023](ADR-023-elf-pt-load-loader.md)) already parses ELF64 little-endian AArch64 **`ET_EXEC`**, maps **`PT_LOAD` only**, **rejects `PT_INTERP`**, rejects W+X, and `ERET`s to `e_entry`. A2–A4 and A9 reuse that parser on FAT `/hello` ([ADR-030](ADR-030-os-app-slots.md), [ADR-032](ADR-032-track-a-leftovers.md)). The in-tree payload is `libctos`, not musl or glibc.

This mile is **docs only**. It does not change `src/loader.rs`. File presence of this page is not a Linux ELF probe and not dynamic-linker support.

## Probe (what Verified means here)

| Claim | Probe | Status |
| --- | --- | --- |
| Linux ELF types / `PT_*` | Read Linux **v6.10** `include/uapi/linux/elf.h` (`ET_EXEC` 2, `ET_DYN` 3, `PT_LOAD` 1, `PT_DYNAMIC` 2, `PT_INTERP` 3, `PT_PHDR` 6, `PT_TLS` 7) | Document inspection |
| Linux auxv keys | Read Linux **v6.10** `include/uapi/linux/auxvec.h` (`AT_NULL` … `AT_EXECFN`, `AT_MINSIGSTKSZ`) | Document inspection |
| ctos freestanding loader | Read `src/loader.rs` (`parse_elf64`, `PT_INTERP` reject, `LOADER_STACK_VA`) + [ADR-023](ADR-023-elf-pt-load-loader.md) on this tree | Document inspection |
| Dynamic Linux ELF / musl / glibc on the guest | No such serial marker | **Not claimed.** Ledger “Guest runs host apps” stays **Planned**. |

A later kernel `qemu-smoke` is a different row. This mile did not edit `src/`.

## Hypotheses

| ID | Option | Notes |
| --- | --- | --- |
| **H1 (chosen)** | Document the Linux vs ctos gap. Keep the ADR-023 loader **reusable** as the freestanding contract. Do **not** accept `PT_INTERP`. Do **not** claim musl/glibc/dynamic ELF. | Matches #44 / ADR-031. Leaves B6 room to choose never. |
| **H2 (rejected)** | Accept `PT_INTERP` / `ET_DYN` / auxv in this mile, or rewrite `parse_elf64` as a Linux `binfmt_elf`. | ADR-031 forbids accepting `PT_INTERP` here. Would break A3 `Interp` tests and A2–A9 markers. |
| **H3 (rejected)** | Treat A3 `PT_LOAD` + A9 FAT `/hello` as Linux `execve` / dynamic linking. | The loader rejects `PT_INTERP`. Standing EL0 is not `execve`. Honesty fail (NFR-06). |
| **H4 (rejected)** | Promise a musl or glibc port, “runs Alpine,” or a guest `ld-linux` / `ld-musl`. | ADR-031 H2. No probe exists. |
| **H5 (rejected)** | Fork a second incompatible parser this mile so Track A and “Linux ELF” diverge silently. | #44 asked to keep the Track A loader reusable. A later Linux path, if B6 ever names one, is a **new layer** that may call the `PT_LOAD` helpers — not a silent retarget. |

## Decision

1. **What Linux `exec` of an ELF does (inspection).** Linux `fs/binfmt_elf.c` (not copied here) loads `ET_EXEC` or `ET_DYN` (PIE). If a `PT_INTERP` string is present, it also maps that interpreter (typically glibc `ld-linux-aarch64.so.1` or musl `ld-musl-aarch64.so.1`) and **transfers control to the interpreter entry**, not the binary `e_entry`. The kernel builds a user stack:

   `argc` → `argv[]` → `NULL` → `envp[]` → `NULL` → `auxv[]` (`AT_*` key/value pairs) → `AT_NULL`.

   The auxiliary vector (Linux v6.10 `auxvec.h`) is how the dynamic linker and libc CRT find program headers, page size, interpreter base, the real entry, and entropy. Typical keys a musl/glibc `_start` reads: `AT_PHDR` / `AT_PHENT` / `AT_PHNUM`, `AT_PAGESZ`, `AT_BASE`, `AT_ENTRY`, `AT_UID`/`AT_EUID`/`AT_GID`/`AT_EGID`, `AT_SECURE`, `AT_RANDOM` (16 bytes), `AT_HWCAP` / `AT_HWCAP2`, `AT_CLKTCK`, `AT_PLATFORM`, `AT_EXECFN`, often `AT_SYSINFO_EHDR` (vDSO) and `AT_MINSIGSTKSZ`.

   After that, the interpreter walks `PT_DYNAMIC` (`DT_NEEDED`, `DT_SYMTAB`, `DT_RELA` / `DT_REL`, `DT_INIT_ARRAY`), maps shared objects, applies relocations, sets up `PT_TLS`, then jumps to `AT_ENTRY`. That is **dynamic linking**. It is not `PT_LOAD` copy.

2. **What the ctos loader does (inspection).** `src/loader.rs` / ADR-023:

   | Item | ctos today |
   | --- | --- |
   | File type | ELF64 LE AArch64 **`ET_EXEC` only** (`NotExec` on anything else, including `ET_DYN`) |
   | Segments | **`PT_LOAD` only.** `PT_PHDR` / `PT_GNU_STACK` / notes ignored. |
   | `PT_INTERP` | **`ParseError::Interp`.** Fail-closed. Smoke/tests require the reject. |
   | W+X | Reject (segment flags or page-union). `SCTLR.WXN` is on ([ADR-015](ADR-015-ro-nx-text-data.md)). |
   | VA window | `paging::MAP_WINDOW` `0x8000_0000` … `+ MAP_WINDOW_SIZE` (512 × 4 KiB). Caps: 8 `PT_LOAD`s, 8 pages. |
   | Stack | One zeroed EL0-RW NX page at `LOADER_STACK_VA` (`0x80007000`). No `argc` / `argv` / `envp` / auxv. |
   | Entry | `ERET` to the binary **`e_entry`**. No interpreter. |
   | CRT | `libctos` wrappers + `_start`. Does not read auxv. Calls `SVC #<n>` ([ADR-021](ADR-021-svc-syscall-abi.md)). |
   | Image source | FAT `/hello` (A9 / ADR-032). Not `execve`. |

   **No Linux ELF feature in the gap table below is `present`.** `PT_LOAD` of a freestanding `ET_EXEC` is **partial** (related map, different contract).

3. **Status words (same idea as B2).**

   | Status | Means |
   | --- | --- |
   | **present** | Same Linux ELF/auxv/`PT_INTERP` meaning, so a musl/glibc AArch64 binary could start with no translation. |
   | **partial** | Track A has a related `PT_LOAD` / stack / entry behavior. Type, stack, or control flow still differ. |
   | **absent** | No analog. B6 may still discuss a *named* subset. Not a promise to implement. |
   | **stay-rejected** | A3 fail-closed reject. This ADR does **not** flip it. Reopen only after B6 names a path, in a later epic. |
   | **never-per-ADR-031** | Would imply a Linux distro / glibc-musl product / guest interpreter that ADR-031 already ruled out as a Track B *promise*. |

4. **Gap table.**

   ### Image type and program headers

   | Linux / ELF item | ctos status | Notes |
   | --- | --- | --- |
   | ELF64 LE AArch64 magic | **partial** | Same header walk. Not a Linux ABI. |
   | `ET_EXEC` `PT_LOAD` map + BSS zero | **partial** | A3 does this inside the 2 MiB window, 8 segments / 8 pages. |
   | `ET_DYN` / PIE / loader-chosen base | **absent** | `parse_elf64` returns `NotExec`. Linux PIE is `ET_DYN`. |
   | `PT_INTERP` | **stay-rejected** | A3 tests require `Interp`. ADR-031: do not accept it in this child. |
   | Load the interpreter ELF | **absent** | No second image, no `AT_BASE`. |
   | Transfer to interpreter entry | **absent** | `ERET` is always binary `e_entry`. |
   | `PT_DYNAMIC` / `DT_NEEDED` | **absent** | Ignored (not `PT_LOAD`). No shared-object search. |
   | Relocations (`DT_RELA` / `REL` / `RELR` / IFUNC) | **absent** | Freestanding image is already linked. |
   | `PT_TLS` / TPIDR_EL0 | **absent** | |
   | `PT_GNU_STACK` exec-stack | **absent** | Ignored. Stack is NX. A Linux exec-stack request would fight W^X. |
   | `PT_GNU_RELRO` | **absent** | |
   | `PT_PHDR` usable at runtime | **absent** | No `AT_PHDR`. Headers are not guaranteed mapped at a libc-known VA. |

   ### Stack and auxv (Linux v6.10 `auxvec.h`)

   | Item | ctos status | Notes |
   | --- | --- | --- |
   | `argc` / `argv[]` / `envp[]` | **absent** | Stack page is zeros. `libctos` does not read `SP`. |
   | `AT_NULL` terminator + key/value pairs | **absent** | |
   | `AT_PHDR` / `AT_PHENT` / `AT_PHNUM` | **absent** | musl/glibc ldso needs these. |
   | `AT_PAGESZ` | **absent** | Guest pages are 4 KiB; nothing publishes that to EL0. |
   | `AT_BASE` | **absent** | Interpreter load base. |
   | `AT_ENTRY` | **absent** | Real program entry after ldso init. |
   | `AT_RANDOM` (16 bytes) | **absent** | Typical stack canary / SSP. No `getrandom` either (B2). |
   | `AT_UID` / `AT_EUID` / `AT_GID` / `AT_EGID` / `AT_SECURE` | **absent** | No uid model. |
   | `AT_HWCAP` / `AT_HWCAP2` / `AT_PLATFORM` | **absent** | |
   | `AT_EXECFN` | **absent** | A9 path is FAT `/hello`, not a Linux pathname. |
   | `AT_SYSINFO_EHDR` (vDSO) | **absent** | No vDSO mapping. |
   | `AT_MINSIGSTKSZ` | **absent** | No POSIX signals (B2). |

   ### musl-linked vs freestanding (the #44 question)

   | Workload | What it still needs on ctos | Honesty |
   | --- | --- | --- |
   | In-tree `libctos` `ET_EXEC` | Already the A3/A9 path. Keep it. | **Not** Linux. |
   | **Static musl** (`-static`, often no `PT_INTERP`) | Linux stack + auxv; Linux `svc #0`/`x8` (`exit_group`, `set_tid_address`, `brk`/`mmap`, `write` on fd 1 — B2); often `ET_DYN` static-PIE; VAs usually **not** in `0x8000_0000`…+2 MiB; image larger than 8 pages. | **Not a small subset.** ADR-031 rejected “runs musl” as a promise. |
   | **Dynamic musl** (`PT_INTERP` → `ld-musl-aarch64.so.1`) | Everything static needs, **plus** accept `PT_INTERP`, load the interpreter, `AT_BASE`/`AT_PHDR`/`AT_ENTRY`, `PT_DYNAMIC` + `DT_NEEDED` files on a real FS, relocations, TLS. | **stay-rejected** + **absent**. Do not claim dynamic Linux ELF. |
   | **glibc** dynamic | Same class as dynamic musl, usually heavier (`ld-linux`, vDSO, more CRT syscalls). | Same non-claim. |

5. **Keep the Track A loader reusable.** A2 flatten, A3 map+`ERET`, A4 standing task, and A9 FAT `/hello` all call `parse_elf64` / `run_image`. This ADR does **not** widen that function. A later Linux ELF path — only if [B6 / #46](https://github.com/artofdream/ctos/issues/46) chooses a compat layer, and only with a probe — would be a **new** module that may reuse page-map helpers. It must not silently retarget `parse_elf64` to accept `PT_INTERP` or `ET_DYN`. Track A tests that reject `Interp` stay.

6. **Not claiming dynamic Linux ELF support.** Say that sentence on the Track B page. Today:

   - A3 maps freestanding ELF64 `ET_EXEC` `PT_LOAD` and **rejects `PT_INTERP`**.
   - The stack has no auxv.
   - `ERET` is not interpreter start.
   - A9 FAT `/hello` is not `execve`.
   - The ledger row “Guest runs host apps (Linux ELF / shell / Python)” stays **Planned**.

7. **If a subset is ever useful (not a decision).** Research hint for B6, not a backlog:

   1. In-tree `libctos` apps need **no** Linux ELF features. Keep ADR-023.
   2. Static musl is already a libc-shaped surface (B2) **plus** auxv/stack/`ET_DYN`/VA-window work. That is not a B4 implementation item.
   3. Dynamic musl/glibc starts with **accepting `PT_INTERP`**. This track’s frame forbids that in B4. B6 may choose **never**.
   4. Do not start with relocations, TLS, or a guest `ld.so`.

8. **Non-goals.** Full Linux ELF ABI; glibc/musl ports; accepting `PT_INTERP` in `src/`; rewriting Track A probes; guest containers ([ADR-029](ADR-029-containers-nongoal.md)); new FR/NFR IDs; author self-merge ([ADR-002](ADR-002-pr-identity-split.md)).

## Consequences

- Docs: this ADR. [track-b.md](../04-roadmap/track-b.md) B4 becomes **Documented**. [SUMMARY.md](../SUMMARY.md) and [roadmap.md](../04-roadmap/roadmap.md) link here. Site extras point at the gap without a userspace claim.
- B3 / B5 / B6 stay **Planned** research. No `src/` change in this PR.
- Track A A3 remains the product loader for in-tree freestanding apps. `PT_INTERP` stays rejected.
- Honesty: document inspection only. Do not add a Verified Linux-ELF or dynamic-linker ledger row because this file exists.
