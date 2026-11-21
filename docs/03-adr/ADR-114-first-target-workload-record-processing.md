# ADR-114 — First target workload: one EL0 record-processing app

- Status: **Proposed.** Nothing in this document changes a claim. The sponsor decides whether this is the first target workload, and sets the numbers marked *TBD-by-sponsor*. Do not self-merge ([ADR-002](ADR-002-pr-identity-split.md)).
- Date: 2026-11-21 (CET)
- Base: main `f4ca2cee` (after [#177](https://github.com/artofdream/ctos/pull/177)). ADR-113 is the last ADR on main, so this is ADR-114.
- Machine: amd64 workstation (docs only; throwaway clone). No cloud, no CloudAgent, no AWS call, no `kvm-recheck` dispatch.
- Asked by: the user, 2026-11-21, after asking what is needed to run a real workload and which kind of workload ctos targets ("please do").
- Related: [ADR-048](ADR-048-app-hosting-claim-criteria.md) / [ADR-052](ADR-052-sponsor-accept-app-hosting.md) (freestanding app hosting, Verified), [ADR-091](ADR-091-el0-isolated-accept-draft.md) (the accepted "EL0 isolated" sentence), [ADR-036](ADR-036-linux-compat-decision.md) (Linux compat: never), [ADR-029](ADR-029-containers-nongoal.md) (containers: non-goal), [ADR-113](ADR-113-own-arm64-kvm-validation-box.md) (owned arm64 box, Proposed).
- Not touched: `src/`, `user/`, `libctos/`, `Cargo.*`, workflows, the required checks, the ADR-091 rev 4 and ADR-106 sentences.

## Question

ctos has never named a real operational workload. What is the **first** workload it should aim to run, and what does "runs reliably" have to mean, in probes, before anyone may say so?

## Context (what is true today)

- ctos is a research / teaching AArch64 kernel. It is not production-ready, not POSIX, not a container host, and not a "secure OS" ([limits](../overview/limits.md)). A green QEMU smoke is not certification.
- Freestanding app hosting is **Verified** only in the scoped sense of [ADR-048](ADR-048-app-hosting-claim-criteria.md) / [ADR-052](ADR-052-sponsor-accept-app-hosting.md). "EL0 isolated" is **Verified** only as the accepted [ADR-091](ADR-091-el0-isolated-accept-draft.md) sentence (one core, QEMU `virt`, `-cpu cortex-a76`, TCG; not real hardware).
- There is no libc, no POSIX, no dynamic linker, no process model with a public ABI. Linux compatibility is **never** ([ADR-036](ADR-036-linux-compat-decision.md)); containers are a **non-goal** ([ADR-029](ADR-029-containers-nongoal.md)). Existing Linux / Windows / macOS apps cannot run. An app has to be rebuilt as `no_std` Rust against `libctos`.
- The in-tree EL0 samples (`/hello`, `/fsdemo`, `/fatdemo`, `/yldemo`, `/netdemo`, `/udpdemo`, `/mkdemo`, `/tcpdemo`) are probes, not workloads ([what can run](../overview/what-can-run.md)).
- **Virtual SError** under KVM is Verified once on Graviton metal ([ADR-112](ADR-112-b3-graviton-real-hardware-serror-plan.md), run #5). That is a fault-path probe, not a workload.

### What the app could use today (read from `main` `f4ca2cee`)

Public EL0 ABI in `libctos` (`SYS_*` 16–28): `exit`, `uart_write`, `yield`, `fs_create`, `fs_open`, `fs_read`, `fs_write`, `fs_close`, `fs_mkdir`, and four fixed network probes (`net_mac`, `net_ping`, `net_udp_dns`, `net_tcp_echo`).

Storage behind the thin VFS ([ADR-027](ADR-027-thin-vfs-memfs.md), prefix mounts [ADR-058](ADR-058-vfs-prefix-mounts.md)): `/mem…` is in-RAM memfs; everything else under `/` is FAT16 on virtio-blk. FAT16 has write ([ADR-050](ADR-050-fat16-write.md)), root readdir ([ADR-056](ADR-056-fat16-readdir.md)), delete ([ADR-057](ADR-057-fat16-delete.md)), multi-cluster grow ([ADR-064](ADR-064-fat16-multi-cluster-grow.md)), mkdir ([ADR-073](ADR-073-fat16-mkdir.md)) and nested mkdir + empty rmdir ([ADR-077](ADR-077-fat16-nested-rmdir.md)). FAT32 is a separate, **read-only**, kernel-side reader on a second disk and is **not** mounted behind the VFS ([ADR-107](ADR-107-fat32-read-second-virtio-blk.md)).

### Gaps that matter for this workload (stated, not fixed here)

| Gap | Today on `main` | Why it matters |
| --- | --- | --- |
| File size | `vfs::FILE_MAX` = 1024 bytes; FAT16 grow refuses `end > FILE_MAX` (ADR-064) | Input and output files are capped at 1 KiB. |
| Per-call I/O | `FS_IO_MAX` = 64 bytes per `fs_read` / `fs_write` | Fine for a loop; must be measured for the performance target. |
| Path grammar | `PATH_MAX` = 16; flat `/name` or one nested `/parent/child`; FAT 8.3 names | File naming must fit. |
| Open handles | 4 FAT handles, 4 memfs handles | One input + one output fits. |
| File position | Offset starts at 0 on `open`; no seek, no truncate | Rewrite-in-place and resume need new ops or a new-file convention. |
| EL0 syscalls not exposed | `unlink`, `readdir`, `rmdir` exist in the kernel VFS but have no `SYS_*`; there is no `rename`, no `fsync`, no `stat` | No atomic replace (write-temp-then-rename) and no durable commit point. |
| Memory | Loader maps at most 8 `PT_LOAD` pages per image, plus one stack page; `libctos` has no allocator and there is no EL0 heap / `mmap` | The app must work in fixed buffers; "memory exhaustion" has to be defined against these limits. |
| App crash | An unexpected lower-EL sync exception prints `exception: unhandled lower sync esr=… elr=…` and **parks the kernel** | Today one app fault stops everything. Fail-closed, but not survivable. |
| Interrupted write | No journal, no FAT mirror check, write ordering of data / FAT / dirent not documented as crash-safe | Behaviour after power loss mid-write is **Unknown**. |
| Restart / supervision | No supervisor, no task restart, no watchdog | "Recovery" has no mechanism yet. |
| Resource limits | No per-task CPU, memory or I/O quotas | Soak and isolation tests have nothing to enforce. |

These are read from source and the ADRs above; none of them is a new probe.

## Decision (proposed)

**The first target workload is one small, purpose-built record-processing app running at EL0:** it reads records from a FAT16 file, validates and transforms each record, and writes the results (and a count of rejected records) to a second FAT16 file, then exits with a status.

Why this one:

- It does useful, checkable work (input → expected output) without Python, TLS, a database, a dynamic linker, or a production network stack.
- It exercises exactly the surfaces ctos already has (EL0, `libctos`, VFS, FAT16) and pushes on the gaps in the table above, which is where the next honest work is.
- Its correctness is a byte-for-byte comparison, which fits the fail-closed smoke style.

Out of scope here: a **telemetry collector** (UDP/TCP ingest) is a plausible later follow-up, but it needs real network hardening (arbitrary ports, timeouts, malformed packets) well beyond the four fixed network probes. It gets its own ADR if wanted. Ported third-party apps, POSIX, Linux compat and containers stay out.

## What "runs reliably" means (acceptance criteria)

Each criterion becomes a probe / smoke marker in `scripts/qemu-smoke.sh` (or a `#[test_case]`) **before** any claim. Until a criterion's marker passes on CI, its ledger row stays **Planned**. Numbers below are labelled: *proposed* (conservative, from today's limits) or *TBD-by-sponsor*.

### R1 — Defined workload

- **Input format** (*proposed*): ASCII, one record per line, `\n`-terminated, fixed fields `id,value` where `id` is 1–8 decimal digits and `value` is a signed decimal integer that fits `i32`. Anything else is a rejected record, never a crash.
- **Transform** (*proposed*): output `id,value,checksum` for each valid record (checksum a documented, simple function, e.g. CRC-16 or a sum mod 65536), plus a trailer line `ok=<n> rejected=<m>`.
- **Expected output**: a golden output file committed next to the golden input; the smoke compares bytes exactly.
- **Data size** (*proposed*): first cut fits today's `FILE_MAX` (≤ 1024 bytes in, ≤ 1024 bytes out, about 40–60 records). A larger size is *TBD-by-sponsor* and needs the `FILE_MAX` gap closed first.
- **Performance target**: **measure first.** Record CNTPCT ticks for the whole run (same method as [ADR-046](ADR-046-slot-perf-delta.md)) on the default smoke machine and report it. A pass/fail threshold is *TBD-by-sponsor* after at least one measured baseline; TCG timings are not hardware timings.
- Marker sketch: `work: ok records=<n> rejected=<m> ticks=<t>`; any mismatch prints `work: fail reason=…` and fails the smoke.

### R2 — App ported via `libctos`

- The app is a `no_std` Rust EL0 ELF under `user/`, built like the existing samples and published on the FAT16 image, loaded through the existing loader (no production `include_bytes!`, per ADR-048 criterion 3).
- Missing ops are added one finding per PR, each with its own probe, only as far as R1–R4 need. Likely list, in order: raise or remove the 1 KiB `FILE_MAX` cap; an EL0 `unlink` (and possibly `rename`) so the app can write a temp file and replace the result; a defined durability point (an `fsync`-like call or a documented write-through guarantee); possibly a small fixed EL0 buffer arena instead of a heap. Each is Planned until its probe is Verified.

### R3 — Failure tests (fail closed)

Each is a negative smoke case with a named marker. "Survives" means: the kernel keeps running and reports the failure, and (once two apps can run) a second, independent app still completes its own golden run.

1. **Malformed input**: truncated line, over-long field, non-digit, overflow, empty file, missing file, file over the size limit. Expected: rejected count rises or a clean `work: fail reason=input` exit; never a kernel park; never a partial output presented as complete.
2. **Memory exhaustion**: input that would overflow the app's fixed buffers, and a deliberately over-sized image (more than 8 load pages). Expected: the app refuses with a named reason, or the loader refuses (`TooManyLoads`); never a write outside the app's pages.
3. **App crash / fault**: the app takes a deliberate data abort (bad pointer) and a deliberate `brk`. **Today this parks the kernel** (see gaps). The criterion is: the kernel records `esr` / `elr` / `far`, tears the task down, and keeps running; another app survives. This needs a new kernel mile and is Planned.
4. **Interrupted write**: stop QEMU mid-write (host-side, between `fs_write` calls) and reboot on the same disk image. Expected: either the old output or the complete new output is visible, never a half-written file accepted as valid; the host FAT checker (`scripts/mkfat16.py --check`, or `fsck.fat -n`) reports the image consistent or names the damage. Until a durability mechanism exists this is expected to show the gap, and the ledger row says **Failed** or **Unknown** honestly, not Verified.
5. **Hardware-level faults**: a **virtual SError** taken during the run still follows the existing fail-closed path ([ADR-053](ADR-053-taken-serror-hard-stop.md) / [ADR-112](ADR-112-b3-graviton-real-hardware-serror-plan.md)); this ADR does not claim survival of a virtual SError, only that it is not silently ignored.

### R4 — Soak, limits, diagnostics, restart

- **Soak** (*proposed*): run the golden workload in a loop on the default smoke machine, starting at 1,000 consecutive runs with zero mismatches and zero parks; duration and count for a longer soak are *TBD-by-sponsor*. CI keeps a short version; the long soak runs on-prem.
- **Resource limits**: define and enforce per-app limits for CPU time (timer-based, using the existing lower-EL IRQ path), pages, open handles and output bytes; each limit has a test that exceeds it and sees a named refusal.
- **Diagnostics**: on any failure the serial log carries one machine-parsable line with the reason, task, `esr`/`elr`/`far` where relevant, and a run counter. No secrets, no host identifiers ([ADR-093](ADR-093-public-identifier-scrub-guard.md) / [ADR-101](ADR-101-host-user-name-scrub-guard.md)).
- **Restart / recovery**: a tested path where, after an app fault or an interrupted write, the app is restarted (by the kernel or on next boot), detects incomplete output, and re-runs to the same golden result. "The OS restarts" alone does not meet this.

## Antifragility (the ratchet)

Here **antifragile** means: every failure found — by a test, a soak, a review or a real incident — becomes a **permanent regression test** in the smoke or `#[test_case]` suite in the same PR as its fix, and is never removed to make CI green. That is how the system gets stronger from the failures it meets. Restarting after a crash is recovery, not antifragility, and does not count on its own.

## Hardware

An owned arm64 box ([ADR-113](ADR-113-own-arm64-kvm-validation-box.md), Proposed) would let the full suite, soak included, run on real silicon without a cloud dependency. It helps validation; it does **not** by itself deliver any of R1–R4. Real-hardware results for this workload stay **Unknown** until probed on such a machine.

## Explicit non-claims

- Not production. Not a "secure OS". Not POSIX, not Linux-compatible, not a container host.
- Not a claim that ctos runs existing applications.
- No ledger row flips from **Planned** until its probe is Verified on CI **and** the sponsor accepts the claim wording.
- No change to the ADR-091 sentence, ADR-052, or the Proposed ADR-106 sentence.

## Consequences

- This PR is documentation only. No code, workflow, or required-check change; no cloud spend.
- A new honesty-ledger row "First target workload runs reliably (ADR-114)" is **Planned**.
- Follow-up work is one finding, one PR: R1 golden files + app, then each gap in the table, then R3 negatives, then R4. Each PR names its marker.
- If the sponsor picks a different first workload, this ADR is superseded, not edited into a claim.
