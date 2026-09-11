# Advantages

These are properties of **how the repo is run**, not a product pitch. They are useful if you want a small AArch64 kernel you can study without inflated status.

## Document-first

Vision → architecture → ADR → roadmap stay ahead of code for each loop ([FR-14](../02-requirements/fr-nfr.md), [NFR-13](../02-requirements/fr-nfr.md)). Frozen IDs are [FR-01–FR-15 and NFR-01–NFR-14](../02-requirements/fr-nfr.md). New IDs are not invented in chat.

## Probed claims

Status words need a command, a serial capture, a `gh run` URL, or a file read that matches the claim ([NFR-06](../02-requirements/fr-nfr.md), [honesty ledger](../framework/honesty-ledger.md)). Unprobed stays **Unknown**. That is slower to write and harder to game than a green README badge.

## QEMU `virt` learning scope

Primary ISA is AArch64 on QEMU `virt` + PL011 ([ADR-003](../03-adr/ADR-003-primary-isa-aarch64.md)). The scope is small on purpose: UART, exceptions, paging, heap, a cooperative scheduler, then pillar cuts. It is not a distro and not a board bring-up until a board probe exists.

## Three pillars as NFRs

After M9, antifragility, security, and performance are first-class ([ADR-011](../03-adr/ADR-011-three-pillars.md), [pillars.md](../framework/pillars.md)). They still need probes. “Secure” and “fast” are not default adjectives.

## Author ≠ merger

[ADR-002](../03-adr/ADR-002-pr-identity-split.md): the author does not merge their own PR. `artofdream` vs `cursor[bot]`. Same-login Approve is still self-review.

## Fail-closed sensors

`scripts/qemu-smoke.sh` and `force-fail` are meant to break when a marker disappears. Repeated host misses become ratchets, not extra paragraphs of advice ([antifragility.md](../framework/antifragility.md)).

## Immutability

**Scoped yes. Absolute no.**

We already have probed, **scoped** cuts: identity-image RO+NX (`ro: ok`, [ADR-015](../03-adr/ADR-015-ro-nx-text-data.md)) and identity `.text` tear after a high-VA rewrite (`ident: range` / `ident: live`, [ADR-019](../03-adr/ADR-019-identity-text-range-tear.md), [ADR-020](../03-adr/ADR-020-identity-fnptr-reloc.md)). Those are page-table facts on this QEMU `virt` guest.

They are **not** “the kernel is immutable,” “W^X everywhere,” or a secure-boot product. `.rodata` / `.data` / heap can still be identity-mapped. Future mappings are not automatically covered. Say a narrower sentence only when the [honesty ledger](../framework/honesty-ledger.md) has a matching probe.

Limits of this shape: [Drawbacks / limits](limits.md).
