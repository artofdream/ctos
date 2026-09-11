# ctos docs

Learning-kernel documentation for [ctos](https://github.com/artofdream/ctos): a minimal bare-metal **AArch64** OS in Rust. Primary path is QEMU `virt` and PL011 UART ([ADR-003](03-adr/ADR-003-primary-isa-aarch64.md)).

This is **not** a product marketing site. Status words need a probe. Unprobed stays **Unknown**. Do not say “secure OS,” “production ready,” or “EL0 isolated.”

## Overview

| Page | What it answers |
| --- | --- |
| [How we measure](overview/measure.md) | Performance, stability, and what “apps” we can honestly claim |
| [Prerequisites](overview/prerequisites.md) | Nightly Rust, QEMU `virt`; Pages not required for kernel work |
| [Why this shape](overview/advantages.md) | Document-first, probed claims, QEMU scope, pillars as NFRs |
| [Limits](overview/limits.md) | Learning kernel; identity stub; PAN unclaimed; no net/DMA |

## Deep dives

1. [Product vision](01-vision/product-vision.md)
2. [FR / NFR](02-requirements/fr-nfr.md) — frozen `FR-01`–`FR-15`, `NFR-01`–`NFR-14`
3. [Technical architecture](02-architecture/technical-architecture.md)
4. [Roadmap](04-roadmap/roadmap.md)
5. [Honesty ledger](framework/honesty-ledger.md)
6. [Three pillars](framework/pillars.md)

Kernel build: [GitHub README](https://github.com/artofdream/ctos#readme). Session protocol: [AGENTS.md](https://github.com/artofdream/ctos/blob/main/AGENTS.md).

## URLs

| URL | Honesty |
| --- | --- |
| `https://ctos.artof.link` | Route 53 CNAME is **in place** (see [website.md](website.md)). HTTPS serving these docs is **Planned** until Pages is enabled and a fetch succeeds. |
| `https://artofdream.github.io/ctos/` | Project-site fallback. **Unknown** until a `pages` workflow on `main` is green. |
