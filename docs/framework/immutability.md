# Immutability (scoped, not absolute)

**Compatible** with ctos principles only as **scoped** immutability: code and other RO regions stay non-writable after a probed lock-down; later, loaded app images can be RO too. That lines up with security (NFR-10), honesty (NFR-06), and antifragility (NFR-05) — a region is RO when a probe says so, and a Failed write-to-RO stays in the ledger.

**Incompatible** if absolute. A kernel must mutate heap, page tables, device MMIO, and task state. “Immutable OS” as marketing is a status inflation. Do not write it.

Hub: [overview.md](overview.md). Pillars: [pillars.md](pillars.md). Threat model: [security.md](security.md).

Plan issues (open on 2026-09-11): [Track A #31](https://github.com/artofdream/ctos/issues/31) (freestanding app hosting), [Track B #40](https://github.com/artofdream/ctos/issues/40) (Linux-compat research; not a promise to become Linux). Optional later slot / root-image work: [A9 #48](https://github.com/artofdream/ctos/issues/48).

## Already practiced (probed)

These are **scoped** cuts on QEMU `virt`. They are not an immutable kernel.

| Scope | Probe | ADR |
| --- | --- | --- |
| Identity `.text`/`.rodata` RO+X; `.data`/heap RW+NX | `ro: nx data` / `ro: write fault` / `ro: ok` | [ADR-015](../03-adr/ADR-015-ro-nx-text-data.md) |
| `SCTLR.WXN` on | same RO+NX smoke | ADR-015 |
| Live identity `.text` torn after high-VA jump + vtable rewrite | `ident: reloc` / `ident: live` / `ident: ok` | [ADR-020](../03-adr/ADR-020-identity-fnptr-reloc.md) |

Heap, PTEs, UART/GIC, and coop stacks stay writable on purpose. `.rodata`/`.data`/heap identity tear is still **Planned**.

## Track A — RO app payloads (later)

[Track A #31](https://github.com/artofdream/ctos/issues/31) is the freestanding-app plan (SVC ABI, `libctos`, loader, standing EL0 as normal mode). **RO app payloads** belong there: once a loader maps an image into user TTBR0, that image can be RO+X the same way kernel text is today. That is **Planned**. Today there is still **one linked kernel ELF** — no separate app slot, not Verified.

Optional later: an immutable **OS image / root slot** so OS updates and app payloads disconnect ([A9 #48](https://github.com/artofdream/ctos/issues/48)). Needs A1–A4 first. Not a Verified “immutable OS.”

[Track B #40](https://github.com/artofdream/ctos/issues/40) must not use Linux-compat research to claim an immutable or container host. Containers stay a [non-goal](host-apps.md).

## Claim gate (ADR-style, no new ADR here)

Same rule as [ADR-001](../03-adr/ADR-001-honesty-harness-for-ctos.md) / [ADR-011](../03-adr/ADR-011-three-pillars.md):

1. Name the **scope** (which pages, which image).
2. Cite a **probe** (serial + `#[test_case]`, or source absence).
3. Only then **Verified**. Unprobed stays **Unknown**. Future work stays **Planned**.
4. Do not say “immutable OS,” “W^X kernel” as a product sentence, or “secure because RO.”

File presence of this note is not that probe. See the [honesty ledger](honesty-ledger.md).
