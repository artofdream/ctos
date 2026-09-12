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

# Roadmap

- [Roadmap](04-roadmap/roadmap.md)
- [Track A (subordinate)](04-roadmap/track-a.md)
- [Track B (subordinate)](04-roadmap/track-b.md)

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
