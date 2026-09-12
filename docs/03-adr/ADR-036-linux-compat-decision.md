# ADR-036 — Track B decision: compat layer vs reimplement vs never

- Status: Accepted (**never** — research ladder complete; no Linux-compat implementation track)
- Date: 2026-09-12

## Context

Track B child [B6 / #46](https://github.com/artofdream/ctos/issues/46) (parent [#40](https://github.com/artofdream/ctos/issues/40)) asks for an explicit decision after the research ladder:

1. thin Linux-compat **layer** on top of ctos,
2. **reimplement** Linux-like surfaces in ctos identity, or
3. **never** (stop at research docs).

Frame and evidence already on `main`:

| Mile | Artifact | Honest readout |
| --- | --- | --- |
| B1 | [ADR-031](ADR-031-linux-compat-goals.md) | ABI **subset** research only; **not claiming Linux userspace**; containers stay [ADR-029](ADR-029-containers-nongoal.md) |
| B2 | [linux-aarch64-syscall-gap.md](../research/linux-aarch64-syscall-gap.md) | **No** Linux syscall is `present` (convention + number space miss); Track A SVCs are **partial** |
| B3 | [ADR-035](ADR-035-process-model-standing-el0.md) | Standing EL0 ≠ `clone`/`execve`/`wait4`; do not add those syscalls |
| B4 | [ADR-033](ADR-033-linux-elf-auxv-pt-interp.md) | Freestanding loader stays; **`PT_INTERP` stay-rejected**; dynamic musl/glibc not a small subset |
| B5 | [ADR-034](ADR-034-linux-vfs-vs-thin-ctos.md) | Thin `VfsOps` ≠ Linux VFS; mount/overlay **never-per-ADR-031** |
| B7 | [ADR-029](ADR-029-containers-nongoal.md) | Guest OCI/Docker already **never** |

Track A A1–A9 (+ leftovers [ADR-032](ADR-032-track-a-leftovers.md)) is the probed freestanding product path. Core principles ([principles.md](../framework/principles.md)) outrank Track B.

This mile is **docs only**. It does not change `src/`. File presence of this ADR is not a Linux userspace probe.

## Hypotheses

| ID | Option | Notes |
| --- | --- | --- |
| **H1 (rejected)** | Thin Linux-compat **layer** (translate Linux AArch64 `svc #0` / `x8` onto ctos, grow auxv/`PT_INTERP`/process table behind a flag) | B2–B5 show the gap is **wide** (syscalls, process model, dynamic ELF, VFS). A layer would become a second ABI to honesty-ledger and fail-closed smoke. No named probed workload justified the cost. |
| **H2 (rejected)** | **Reimplement** Linux-like surfaces as native ctos (Linux numbers/names in `src/`, still “ctos”) | Same gap surface as H1, plus a silent retarget of Track A SVC `#n` / `libctos` / reject-`PT_INTERP` markers. Conflicts with ADR-031 “keep freestanding Track A.” |
| **H3 (chosen)** | **Never** — Track B stops as a **completed research ladder**. No Linux-compat implementation epic from these ADRs. | Matches honesty (NFR-06), antifragility (do not grow an unprobed ABI), and “principles drive.” Leaves the door open only via a **new** epic + ADR with a named workload and probe — not by treating B2–B5 as a backlog. |

## Decision

1. **Choose never.** ctos will **not** start a Linux-compat layer or a Linux-like reimplementation from Track B B1–B5. Track B’s job was to map the gap honestly. That job is **done**.

2. **What “never” means here.**
   - Do **not** treat the B2 syscall table, B3 process gap, B4 ELF/auxv gap, or B5 VFS compare as an implementation backlog.
   - Do **not** accept `PT_INTERP`, Linux `x8` numbers, `clone`/`execve`/`wait*`, POSIX VFS, or guest containers “later on Track B.”
   - Do **not** change the ledger row “Guest runs host apps (Linux ELF / shell / Python)” from **Planned** to a softer promise. It stays a distant possible future only if a **new** epic says so with a probe.
   - Track A freestanding apps (SVC / `libctos` / `PT_LOAD` / standing EL0 / thin VFS / FAT slots) remain the supported path.

3. **What remains allowed.**
   - Keep and cite B1–B5 / B7 docs as the honesty record of *why* Linux userspace is out.
   - Learn from Linux **ideas** in future freestanding miles only when they fit ctos pillars and get their own ADR + probe (not “compat”).
   - Reopen Linux-compat only with a **new GitHub epic**, a new ADR that names a concrete workload + probe, and an explicit override of this decision. Do not mint FR/NFR IDs here.

4. **Relationship to ADR-029 / ADR-031.** Containers stay a non-goal. ADR-031’s frame (“B6 may choose never”) is exercised by this ADR. Principles still drive over tracks.

## Consequences

- [track-b.md](../04-roadmap/track-b.md) marks B6 **Decided (never)** and the research ladder **complete**.
- Plan issue [#40](https://github.com/artofdream/ctos/issues/40) may close once this ADR merges (children B1–B7 documented/decided).
- No `src/` change. No new FR/NFR IDs. No self-merge ([ADR-002](ADR-002-pr-identity-split.md)).
- Future “runs musl/Alpine/Linux ELF” headlines without a new epic + probe are honesty failures.

## Probe (what Verified means here)

| Claim | Probe | Status |
| --- | --- | --- |
| B1–B5 / B7 evidence cited above exists on this tip | Read ADR-029/031/033/034/035 + B2 gap note + track-b on `main` | Document inspection |
| This ADR records an explicit **never** decision | Read this file’s Decision section | Document inspection |
| Linux userspace / compat layer on the guest | No serial marker; not claimed | **Not claimed** |