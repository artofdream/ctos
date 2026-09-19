# Summary

[ctos (ctsOS)](index.md)

# Overview

- [What can run today](overview/what-can-run.md)
- [Prerequisites](overview/prerequisites.md)
- [Building or porting](overview/porting.md)
- [Filesystem (memfs + FAT16)](overview/filesystem.md)
- [Hosting apps / containers](overview/hosting-apps.md)
- [KPIs / how we measure](overview/measure.md)
- [Advantages](overview/advantages.md)
- [Drawbacks / limits](overview/limits.md)
- [Archify diagrams (QEMU NMI / SError)](archify/index.md)

# Vision

- [Product vision](01-vision/product-vision.md)

# Requirements

- [FR / NFR](02-requirements/fr-nfr.md)

# Architecture

- [Technical architecture](02-architecture/technical-architecture.md)

# ADRs

- [ADR-001 Honesty harness](03-adr/ADR-001-honesty-harness-for-ctos.md)
- [ADR-002 PR identity split](03-adr/ADR-002-pr-identity-split.md)
- [ADR-003 Primary ISA AArch64](03-adr/ADR-003-primary-isa-aarch64.md)
- [ADR-004 EL1 VBAR + BRK](03-adr/ADR-004-el1-vbar-brk.md)
- [ADR-005 Fatal exception stack](03-adr/ADR-005-fatal-exception-stack.md)
- [ADR-006 GICv2 + generic timer](03-adr/ADR-006-gicv2-generic-timer.md)
- [ADR-007 PL011 UART RX](03-adr/ADR-007-pl011-uart-rx.md)
- [ADR-008 Identity map + frames](03-adr/ADR-008-identity-map-frame-allocator.md)
- [ADR-009 First-fit heap](03-adr/ADR-009-first-fit-heap.md)
- [ADR-010 Cooperative RR on EL1](03-adr/ADR-010-cooperative-rr-el1.md)
- [ADR-011 Three pillars](03-adr/ADR-011-three-pillars.md)
- [ADR-012 W^X NX heap + stacks](03-adr/ADR-012-wx-nx-heap-stacks.md)
- [ADR-013 EL0 isolation direction](03-adr/ADR-013-el0-isolation-direction.md)
- [ADR-014 Linker stack guards](03-adr/ADR-014-linker-stack-guard-pages.md)
- [ADR-015 RO+NX text/data](03-adr/ADR-015-ro-nx-text-data.md)
- [ADR-016 TTBR1 private page](03-adr/ADR-016-ttbr1-private-page.md)
- [ADR-017 TTBR1 high-VA exec](03-adr/ADR-017-ttbr1-high-el1-exec.md)
- [ADR-018 Identity-tear first cut](03-adr/ADR-018-identity-teardown-first-cut.md)
- [ADR-019 Identity text range tear](03-adr/ADR-019-identity-text-range-tear.md)
- [ADR-020 Live identity text tear](03-adr/ADR-020-identity-fnptr-reloc.md)
- [ADR-021 EL0 SVC ABI](03-adr/ADR-021-svc-syscall-abi.md)
- [ADR-022 Freestanding CRT / libctos](03-adr/ADR-022-libctos-crt.md)
- [ADR-023 Guest ELF PT_LOAD loader](03-adr/ADR-023-elf-pt-load-loader.md)
- [ADR-024 Standing EL0 as normal mode](03-adr/ADR-024-standing-el0-normal.md)
- [ADR-025 Identity .rodata tear](03-adr/ADR-025-identity-rodata-tear.md)
- [ADR-026 PAN capability](03-adr/ADR-026-pan-capability.md)
- [ADR-027 Thin VFS + memfs](03-adr/ADR-027-thin-vfs-memfs.md)
- [ADR-028 virtio-blk + FAT16](03-adr/ADR-028-virtio-blk-fat16.md)
- [ADR-029 Guest containers non-goal](03-adr/ADR-029-containers-nongoal.md)
- [ADR-030 OS vs app slots](03-adr/ADR-030-os-app-slots.md)
- [ADR-031 Linux-compat goals](03-adr/ADR-031-linux-compat-goals.md)
- [ADR-032 Track A leftovers](03-adr/ADR-032-track-a-leftovers.md)
- [ADR-033 Linux ELF / auxv / PT_INTERP](03-adr/ADR-033-linux-elf-auxv-pt-interp.md)
- [ADR-034 Linux VFS vs thin ctos VFS](03-adr/ADR-034-linux-vfs-vs-thin-ctos.md)
- [ADR-035 Process model vs standing EL0](03-adr/ADR-035-process-model-standing-el0.md)
- [ADR-036 Linux-compat decision (never)](03-adr/ADR-036-linux-compat-decision.md)
- [ADR-037 Identity .data tear](03-adr/ADR-037-identity-data-tear.md)
- [ADR-038 Identity heap tear](03-adr/ADR-038-identity-heap-tear.md)
- [ADR-039 EL0 entry without VMALLE1](03-adr/ADR-039-el0-entry-without-vmalle1.md)
- [ADR-040 Lower-EL IRQ while standing](03-adr/ADR-040-lower-el-irq-standing.md)
- [ADR-041 IRQ-unmasked default ERET](03-adr/ADR-041-irq-unmasked-default-eret.md)
- [ADR-042 Isolation leftover wrap](03-adr/ADR-042-isolation-leftover-wrap.md)
- [ADR-043 Lower-EL FIQ + SError park](03-adr/ADR-043-lower-el-fiq-serror.md)
- [ADR-044 Taken SError research](03-adr/ADR-044-taken-serror-research.md)
- [ADR-045 Taken SError QMP attempt](03-adr/ADR-045-taken-serror-qmp.md)
- [ADR-046 A9 slot performance delta](03-adr/ADR-046-slot-perf-delta.md)
- [ADR-047 Isolation leftovers decisions](03-adr/ADR-047-isolation-leftovers-decisions.md)
- [ADR-048 App hosting claim criteria](03-adr/ADR-048-app-hosting-claim-criteria.md)
- [ADR-049 Leftover identity RAM tear](03-adr/ADR-049-identity-ram-tear.md)
- [ADR-050 FAT16 write](03-adr/ADR-050-fat16-write.md)
- [ADR-051 FAT vs memfs write CNTPCT](03-adr/ADR-051-fat-memfs-write-cntpct.md)
- [ADR-052 Sponsor accept app hosting](03-adr/ADR-052-sponsor-accept-app-hosting.md)
- [ADR-053 Taken SError hard-stop](03-adr/ADR-053-taken-serror-hard-stop.md)
- [ADR-054 PAN enable lock](03-adr/ADR-054-pan-enable-lock.md)
- [ADR-055 EL0 isolated checklist](03-adr/ADR-055-el0-isolated-checklist.md)
- [ADR-056 FAT16 readdir](03-adr/ADR-056-fat16-readdir.md)
- [ADR-057 FAT16 delete](03-adr/ADR-057-fat16-delete.md)
- [ADR-058 VFS prefix mounts](03-adr/ADR-058-vfs-prefix-mounts.md)
- [ADR-059 fs-libctos sample catalog](03-adr/ADR-059-fs-libctos-sample.md)
- [ADR-060 Isolation leftovers closure checklist](03-adr/ADR-060-isolation-leftovers-closure-checklist.md)
- [ADR-061 fat-libctos FAT-via-VFS sample](03-adr/ADR-061-fat-libctos-sample.md)
- [ADR-062 yield-libctos cooperative yield sample](03-adr/ADR-062-yield-libctos-sample.md)
- [ADR-063 Track N network foundation scope](03-adr/ADR-063-network-foundation-scope.md)
- [ADR-064 FAT16 multi-cluster grow](03-adr/ADR-064-fat16-multi-cluster-grow.md)
- [ADR-065 FAT vs memfs read CNTPCT](03-adr/ADR-065-fat-memfs-read-cntpct.md)
- [ADR-066 virtio-net first frame (N1)](03-adr/ADR-066-virtio-net-first-frame.md)
- [ADR-067 virtio-net ICMP ping (N2)](03-adr/ADR-067-virtio-net-icmp-ping.md)
- [ADR-068 EL0 net SVC + net-libctos sample (N4)](03-adr/ADR-068-el0-net-svc-sample.md)
- [ADR-069 net-ping CNTPCT](03-adr/ADR-069-net-ping-cntpct.md)
- [ADR-070 N3 UDP transport](03-adr/ADR-070-n3-udp-transport.md)
- [ADR-071 N3.x EL0 UDP SVC + udp-libctos sample](03-adr/ADR-071-n3x-el0-udp-svc.md)
- [ADR-072 UDP DNS path CNTPCT](03-adr/ADR-072-udp-dns-cntpct.md)
- [ADR-073 FAT16 mkdir](03-adr/ADR-073-fat16-mkdir.md)
- [ADR-074 N5 thin TCP](03-adr/ADR-074-n5-thin-tcp.md)
- [ADR-075 EL0 fs_mkdir + mkdir-libctos sample](03-adr/ADR-075-el0-fs-mkdir.md)
- [ADR-076 EL0 TCP SVC + tcp-libctos sample](03-adr/ADR-076-el0-tcp-svc.md)
- [ADR-077 FAT16 nested mkdir + empty rmdir](03-adr/ADR-077-fat16-nested-rmdir.md)
- [ADR-078 thin TCP echo CNTPCT](03-adr/ADR-078-tcp-echo-cntpct.md)
- [ADR-079 PAN CPU reopen foundation](03-adr/ADR-079-pan-cpu-reopen.md)
- [ADR-080 PAN enable + EL1-vs-EL0 fault](03-adr/ADR-080-pan-enable-fault.md)
- [ADR-081 Taken SError reopen (still blocked)](03-adr/ADR-081-taken-serror-reopen.md)
- [ADR-082 B1 QEMU TYPE_NMI research (blocker)](03-adr/ADR-082-b1-qemu-type-nmi.md)
- [ADR-083 B1 pinned QEMU TYPE_NMI → el0: serror](03-adr/ADR-083-b1-qemu-nmi-pin.md)
- [Opt-in CI: QEMU NMI pin](dev/qemu-nmi-pin-ci.md)

# Roadmap

- [Roadmap](04-roadmap/roadmap.md)
- [Track A (subordinate)](04-roadmap/track-a.md)
- [Track B (subordinate)](04-roadmap/track-b.md)
- [Track N (subordinate)](04-roadmap/track-n.md)

# Research

- [Linux AArch64 syscall gap (B2)](research/linux-aarch64-syscall-gap.md)

# Framework

- [Core principles](framework/principles.md)
- [Harness map](framework/formula.md)
- [Honesty ledger](framework/honesty-ledger.md)
- [Three pillars](framework/pillars.md)
- [Antifragility](framework/antifragility.md)
- [Security](framework/security.md)
- [EL0](framework/el0.md)
- [SVC ABI](framework/syscall.md)
- [Performance](framework/performance.md)
- [Overview (extra stance)](framework/overview.md)
- [What can run today (extra)](framework/apps-today.md)
- [Building or porting (extra)](framework/building-or-porting.md)
- [Filesystem stance (extra)](framework/filesystem.md)
- [Gaps to host apps (extra)](framework/host-apps.md)
- [Immutability (scoped)](framework/immutability.md)

# Site

- [Docs website + DNS](website.md)
- [Second brain](research.md)