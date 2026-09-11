# Hosting applications — gaps, and containers

What is missing before ctos could **host** an application (a loaded EL0 binary with a stable ABI), and whether it can host **containers**.

Today you **extend the kernel** ([Building or porting](porting.md)). Samples that exist: [What can run today](what-can-run.md). Probes: [honesty ledger](../framework/honesty-ledger.md). Isolation: [el0.md](../framework/el0.md).

No new FR/NFR IDs. Do not say ctos is an app host or a container runtime.

## Gaps before hosting applications

These are missing pieces, not a schedule. Rows without a probe stay **Planned** or unbuilt. “Later” is not a promise.

| Gap | Why it blocks hosting | Status |
| --- | --- | --- |
| **Stable SVC ABI** | Today’s `SVC #1` / `#2` are test miles, not a documented syscall contract | Planned (direction on the porting page) |
| **ELF / user loader** | No loader for a freestanding EL0 binary, let alone a Linux ELF | Not built |
| **Standing EL0 as normal** | Standing enter/leave is a stub mile, not the default way code runs | First mile Verified; normal userspace **Planned** |
| **Stronger isolation** | Umbrella EL0 isolation needs PAN + fuller identity teardown | **Planned** — do not say “EL0 isolated” |
| **VFS + memfs** | No files, no paths; see [Filesystem](filesystem.md) | **Planned** |
| **libctos / CRT** | Nothing to link a freestanding EL0 program against | Not built |
| **Richer I/O** | UART byte in/out only; no TTY, disk, or sockets | UART probed; the rest unbuilt |
| **Preemption / SMP / net** | Cooperative one-CPU yield; no NIC | Later — not a near hosting gate |

Until the first block has probes, “host an application” is a sentence we do not use. That first block is **Track A** (loader + stable ABI + CRT). An OS slot vs app slot — update the kernel without rebuilding in-tree “apps” — waits on Track A. See [Immutability](advantages.md#immutability). Not containers. Not OTA.

## Containers

**No.** ctos cannot host OCI / Docker / Kubernetes workloads.

Those need Linux kernel features (namespaces, cgroups, a Linux ABI, usually overlay/AUFS or equivalent, a rich syscall surface) that this learning kernel **does not have** and is **not aiming at soon**.

**Today the arrow is the other way:** Docker on a host (cts-ai `linux/arm64`) **runs the ctos smoke image**. That is “Docker hosts ctos,” not “ctos hosts containers.” See the [README Docker notes](https://github.com/artofdream/ctos#readme) and the Docker rows in the [ledger](../framework/honesty-ledger.md).

Container support is **not Planned** on this page. Do not add a Planned row unless the sponsor asks for that direction in an ADR.

## Honesty

| Claim | Status |
| --- | --- |
| ctos hosts third-party apps | **No** — gaps above |
| ctos hosts OCI/Docker containers | **No** — not a goal on this page |
| Host Docker runs `ctos-smoke` | Separate ledger row (host tool, not a guest runtime) |
