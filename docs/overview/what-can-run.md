# What can run today

Concrete examples that match **probes already on this tree**. They are in-kernel (or a standing EL0 stub), written against ctos APIs, on QEMU `virt`. They are not third-party applications and not a product runtime.

Status of each probe: [honesty ledger](../framework/honesty-ledger.md). Pillars: [pillars.md](../framework/pillars.md). How we measure: [measure.md](measure.md).

No new FR/NFR IDs. Isolation stays **Planned**. Do not say “apps,” “userspace,” or “secure OS” as if a general-purpose OS existed.

## 1. Cooperative EL1 UART workers

Two tasks on **heap stacks** that print a line and **yield** to each other. Same class as the M9 smoke markers `sched: task a` / `sched: task b` / `sched: ok` ([ADR-010](../03-adr/ADR-010-cooperative-rr-el1.md), [FR-11](../02-requirements/fr-nfr.md)).

A natural variant of the same scheduler is a **serial heartbeat or counter**: one (or both) workers print a tick and yield. Still cooperative EL1. Still UART text. Not preemptive. Not SMP.

## 2. UART RX echo gadget

Read a byte from **PL011 RX** and print it. Same class as the M6 probe `input: rx 0x41` ([ADR-007](../03-adr/ADR-007-pl011-uart-rx.md)).

That is a byte in, a line out. **No TTY, no line editor, no canonical mode, no virtio-keyboard.**

## 3. Standing EL0 stub

A short payload at EL0 that does an **SVC round-trip** and returns. Same class as `el0: standing` / `el0: restored` ([ADR-013](../03-adr/ADR-013-el0-isolation-direction.md), [el0.md](../framework/el0.md)).

This is **not** a process. There is no libc, no files, no argv, no loader for a foreign ELF. Umbrella “EL0 isolated” stays **Planned**.

## What cannot run

Do not imply these work:

- Linux binaries (no Linux ABI, no ELF loader for third-party programs)
- A shell
- Python (or any hosted language runtime)
- Network servers (no NIC, no sockets, no DMA)
- Filesystem apps (no block device, no VFS — [Filesystem (Planned)](filesystem.md))
- SMP workloads (one CPU, cooperative yield only)

Also not claimed: POSIX, GPU, Raspberry Pi, certified security, “production ready,” or **containers** ([Hosting apps / containers](hosting-apps.md)).

How you would add something in-tree (and why Linux apps do not port): [Building or porting](porting.md).
