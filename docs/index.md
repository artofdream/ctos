# ctos docs

This is the **learning-kernel documentation** for [ctos](https://github.com/artofdream/ctos): a minimal bare-metal **AArch64** OS kernel in Rust. Primary path is QEMU `virt` and PL011 UART ([ADR-003](03-adr/ADR-003-primary-isa-aarch64.md)).

It is **not** a product marketing site, a desktop OS, a “secure OS” claim, or a published benchmark. Status words (Verified, Live, Done) need a probe. Unprobed stays **Unknown**. Start with the [honesty ledger](framework/honesty-ledger.md).

## Start here

1. [Product vision](01-vision/product-vision.md) — what ctos is and is not
2. [FR / NFR](02-requirements/fr-nfr.md) — frozen IDs `FR-01`–`FR-15`, `NFR-01`–`NFR-14`
3. [Technical architecture](02-architecture/technical-architecture.md)
4. [Roadmap](04-roadmap/roadmap.md) — one milestone → one branch → one GitHub PR
5. [Three pillars](framework/pillars.md) — antifragility, security, performance

Kernel build and QEMU smoke stay in the [GitHub README](https://github.com/artofdream/ctos#readme). Session protocol: [AGENTS.md](https://github.com/artofdream/ctos/blob/main/AGENTS.md).

## URLs

| URL | Honesty |
| --- | --- |
| `https://ctos.artof.link` | Intended production hostname (Route 53, account `737290977112`). Reachability **Planned** until Pages lists the domain and HTTPS works. |
| `https://artofdream.github.io/ctos/` | GitHub Pages project-site fallback. **Unknown** until a `pages` workflow on `main` is green. |

How to build this book locally, how Pages deploys, and the DNS checklist: [Docs website + DNS](website.md).
