# Hosting applications — gaps, and containers

What is missing before ctos could **host** an application (a loaded user-mode binary with a stable ABI), and whether it can host **containers**.

Today you **extend the kernel** ([Building or porting](porting.md)). Samples that exist: [What can run today](what-can-run.md). Isolation notes: [el0.md](../framework/el0.md). Kernel SVC numbers: [syscall.md](../framework/syscall.md).

Do not say ctos is an app host or a container runtime.

## Today vs the slot split (A9 direction)

**Today:** one linked ELF. Kernel code and any “app-shaped” experiment ship together. QEMU `-kernel` loads that one image.

**Later (Planned):** an **OS slot** (the kernel you update) and an **app slot** (a loaded user-mode binary that can survive an OS swap). That split is the useful meaning of immutability. It depends on Track A (loader + stable ABI + CRT). It is **not** containers and **not** over-the-air firmware updates.

```mermaid
flowchart TD
  TODAY["Today: one linked ELF<br/>kernel + in-tree code<br/>Verified"]
  LATER["Later: OS slot + app slot<br/>update OS without rebuilding apps<br/>Planned — Track A / A9 direction"]
  TODAY -.-> LATER
```

*Do not say the slot split exists. Runtime cost vs “same as today” is unmeasured.*

## Gaps before hosting applications

These are missing pieces, not a schedule. Rows without a probe stay **Planned** or unbuilt. “Later” is not a promise.

A **supervisor call (SVC)** is how user-mode code asks the kernel for help. Reserved `SVC #0`–`#2` stay test miles. A1 documented `exit` / `uart_write` / `yield` ([syscall.md](../framework/syscall.md)). That is not a CRT or loader.

| Gap | Why it blocks hosting | Status |
| --- | --- | --- |
| **Stable SVC ABI** | A1 documented `exit` / `uart_write` / `yield` ([ADR-021](../03-adr/ADR-021-svc-syscall-abi.md)). That is a **kernel** contract, not a loader. | **ABI mile** — Verified only when the ledger has `svc: ok` on this tip. Not app hosting. |
| **libctos / CRT** | A2: `libctos` wrappers + `_start` CRT. Hello is host-built and copied onto the standing EL0 page ([ADR-022](../03-adr/ADR-022-libctos-crt.md)). | **CRT mile** — Verified only when the ledger has `libctos: ok` on this tip. Not a guest loader. |
| **ELF / user loader** | A3: guest ELF64 `PT_LOAD` into user TTBR0, then `ERET` to `e_entry` ([ADR-023](../03-adr/ADR-023-elf-pt-load-loader.md)). A2–A4 load FAT `/hello` (no production embed, [ADR-032](../03-adr/ADR-032-track-a-leftovers.md)). Not a Linux ELF ABI. | **Loader mile** — Verified only when the ledger has `loader: ok` on this tip. Not app hosting. |
| **OS vs app slots** | A9: host kernel ELF + `hello-libctos.elf`; guest FAT `/hello` ([ADR-030](../03-adr/ADR-030-os-app-slots.md)). Same ELF on this OS and `ba6541c` ([ADR-032](../03-adr/ADR-032-track-a-leftovers.md)). | **Slot first cut + leftover** — Verified only when the ledger has `slot: ok` and the cross-update host lines on this tip. Not “apps update independently.” |
| **Standing user mode as normal** | A4: loaded image stands until `exit`; fail-closed restore ([ADR-024](../03-adr/ADR-024-standing-el0-normal.md)). Not a process table. | **Standing-task mile** — Verified only when the ledger has `el0: task-ok` on this tip. Not isolation. Not app hosting. |
| **Stronger isolation** | Umbrella stays Planned/non-claim under [ADR-047](../03-adr/ADR-047-isolation-leftovers-decisions.md) (no PAN enable on a57; `_start` stays; taken SError deferred) | **Planned / non-claim** — do not say “EL0 isolated” |
| **VFS + memfs** | Thin VFS + in-RAM named buffers ([ADR-027](../03-adr/ADR-027-thin-vfs-memfs.md)); see [Filesystem](filesystem.md) | **memfs mile** — Verified only when the ledger has `fs: ok` on this tip. Not POSIX. Not FAT. |
| **On-disk FS** | virtio-blk + FAT16 behind the same VFS ([ADR-028](../03-adr/ADR-028-virtio-blk-fat16.md)) | **block + FAT mile** — Verified only when the ledger has `blk: ok` / `fat: ok` on this tip. Not POSIX. Not writeable FAT. |
| **Richer I/O** | UART byte in/out only; no TTY, disk, or sockets | UART probed; the rest unbuilt |
| **Preemption / extra CPUs / net** | Cooperative one-CPU yield; no NIC | Later — not a near hosting gate |

“Host an application independently of the OS” / “app hosting is done” stays **Planned** until [ADR-048](../03-adr/ADR-048-app-hosting-claim-criteria.md) criteria (documented recipes + slot cross-update + no production embed + honesty about missing Linux/POSIX/containers + sponsor accept). That block is **Track A**. A1–A9 first cuts exist; they are not the product claim. A5: PAN ID-field Verified; enable is non-goal on default cortex-a57 ([ADR-047](../03-adr/ADR-047-isolation-leftovers-decisions.md)). A8 documented rebuild recipes ([what-can-run.md](what-can-run.md)). A9 + leftover: FAT `/hello`, no production embed, cross-update on `ba6541c`. See [Immutability](advantages.md#immutability).

## Containers

**Non-goal.** ctos cannot host OCI / Docker / Kubernetes workloads, and it is **not aiming to**.

Those need Linux kernel features (namespaces, cgroups, a Linux ABI, usually overlay or equivalent, a rich syscall surface) that this learning kernel **does not have**. Decision: [ADR-029](../03-adr/ADR-029-containers-nongoal.md). Track B B7 ([#47](https://github.com/artofdream/ctos/issues/47)).

**Today the arrow is the other way:** Docker on a host (cts-ai `linux/arm64`) **runs the ctos smoke image**. That is “Docker hosts ctos,” not “ctos hosts containers.” See the [README Docker notes](https://github.com/artofdream/ctos#readme) and the Docker rows in the [ledger](../framework/honesty-ledger.md).

```mermaid
flowchart LR
  HOST["Linux host + Docker"] -->|"builds and QEMU-smokes"| CTOS["ctos guest"]
  CTOS -.->|"does not host"| OCI["OCI / Docker / k8s"]
```

*Host harness ≠ guest runtime. File presence of this page is not a container probe.*

Container support is **not Planned** on this page. Do not add a Planned row. Reopen only with a new GitHub epic plus a new ADR — not a Track B win, and not a new FR/NFR ID.

## Claim criteria (ADR-048)

Product “apps run independently / hosting done” may become Verified **only** when all of the following are ledger-Verified and sponsor-accepted ([ADR-048](../03-adr/ADR-048-app-hosting-claim-criteria.md)):

1. Documented freestanding-app recipes (existing markers only).
2. Slot cross-update (same published ELF on this OS and a documented prior OS).
3. No production hello embed (`slot: embed` never appears on the production path).
4. Docs still refuse Linux ABI / POSIX / guest containers ([ADR-029](../03-adr/ADR-029-containers-nongoal.md)).
5. Explicit sponsor accept on the flip PR (no self-merge).

Do **not** claim done now. A1–A9 first cuts alone are insufficient.

## Honesty

| Claim | Status |
| --- | --- |
| ctos hosts third-party apps / “hosting done” | **No** — Planned until [ADR-048](../03-adr/ADR-048-app-hosting-claim-criteria.md) |
| ctos hosts OCI/Docker containers | **No** — **non-goal** ([ADR-029](../03-adr/ADR-029-containers-nongoal.md)) |
| Linux-compat research (Track B) | Frame ([ADR-031](../03-adr/ADR-031-linux-compat-goals.md)) + B2 gap map ([linux-aarch64-syscall-gap.md](../research/linux-aarch64-syscall-gap.md)) + B3 process-model stance ([ADR-035](../03-adr/ADR-035-process-model-standing-el0.md)) + B4 ELF/auxv gap ([ADR-033](../03-adr/ADR-033-linux-elf-auxv-pt-interp.md)) + B5 VFS compare ([ADR-034](../03-adr/ADR-034-linux-vfs-vs-thin-ctos.md)). **Not claiming Linux userspace yet.** **Not claiming dynamic Linux ELF.** **Not claiming a Linux filesystem.** |
| Host Docker runs `ctos-smoke` | Separate ledger row (host tool, not a guest runtime) |
