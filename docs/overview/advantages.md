# Advantages

These are properties of **how the repo is run**, not a product pitch. They are useful if you want a small AArch64 kernel you can study without inflated status.

The landing **Driving principles** block (honesty, antifragility, security, performance, document-first) is the force; Track A/B stay subordinate. Visitor-facing source: [landing](../index.md#driving-principles). Deep pillar notes: [pillars.md](../framework/pillars.md).

## Document-first

Vision → architecture → ADR → roadmap stay ahead of code for each loop (document-first — [FR-14](../02-requirements/fr-nfr.md), [NFR-13](../02-requirements/fr-nfr.md)). Frozen IDs are [FR-01–FR-15 and NFR-01–NFR-14](../02-requirements/fr-nfr.md). New IDs are not invented in chat.

## Probed claims

Status words need a command, a serial capture, a `gh run` URL, or a file read that matches the claim (honesty — [NFR-06](../02-requirements/fr-nfr.md), [honesty ledger](../framework/honesty-ledger.md)). Unprobed stays **Unknown**. That is slower to write and harder to game than a green README badge.

## QEMU `virt` learning scope

Primary ISA is AArch64 on QEMU `virt` + serial UART ([ADR-003](../03-adr/ADR-003-primary-isa-aarch64.md)). The scope is small on purpose: UART, exceptions, paging, heap, a cooperative scheduler, then pillar cuts. It is not a distro and not a board bring-up until a board probe exists.

## Three pillars as requirements

After the cooperative scheduler (M9), antifragility, security, and performance are first-class ([ADR-011](../03-adr/ADR-011-three-pillars.md), [pillars.md](../framework/pillars.md)). They still need probes. “Secure” and “fast” are not default adjectives.

## Author ≠ merger

[ADR-002](../03-adr/ADR-002-pr-identity-split.md): the author does not merge their own PR. `artofdream` vs `cursor[bot]`. Same-login Approve is still self-review.

## Fail-closed sensors

`scripts/qemu-smoke.sh` and `force-fail` are meant to break when a marker disappears. Repeated host misses become ratchets, not extra paragraphs of advice ([antifragility.md](../framework/antifragility.md)).

## Immutability

**Scoped yes. Absolute no.** Two different sentences:

**Page-table scope (probed today).** Some kernel memory is read-only and executable, other memory is writable and not executable (**W^X** on that image slice — write *or* execute, not both). Smoke markers: `ro: ok` ([ADR-015](../03-adr/ADR-015-ro-nx-text-data.md)) and identity `.text` tear (`ident: range` / `ident: live`, [ADR-019](../03-adr/ADR-019-identity-text-range-tear.md), [ADR-020](../03-adr/ADR-020-identity-fnptr-reloc.md)) on this QEMU `virt` guest. Not “the kernel is immutable” or “W^X everywhere.” Claim only with a [ledger](../framework/honesty-ledger.md) probe.

**OS slot vs app slot (direction, not built).** The useful product meaning is: **update the OS without rebuilding the apps**, and the reverse. That needs a separate **OS slot** (the kernel image you `-kernel` today) and an **app slot** (a loaded user-mode binary that survives an OS swap). That slot split **depends on Track A**. A1 is a kernel SVC ABI mile only ([syscall.md](../framework/syscall.md)). Loader + `libctos` stay **Planned** on [Building or porting](porting.md) and [Hosting apps](hosting-apps.md). Until those have probes, there is only one slot: in-tree kernel code.

This is **not** containers, and **not** OTA / A-B firmware updates. Those are later and unclaimed. Do not say “immutable OS updates” until an OS-slot/app-slot probe exists.

Runtime cost vs neutral is **unmeasured**. Measure first; no invented numbers. [KPIs — OS slot vs app slot](measure.md#os-slot-vs-app-slot-performance).

Limits of this shape: [Drawbacks / limits](limits.md).
