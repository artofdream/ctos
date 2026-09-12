# Immutability (scoped, not absolute)

**Compatible** with ctos principles only as **scoped** immutability: code and other RO regions stay non-writable after a probed lock-down; later, loaded app images can be RO too. That lines up with security (NFR-10), honesty (NFR-06), and antifragility (NFR-05) — a region is RO when a probe says so, and a Failed write-to-RO stays in the ledger.

**Incompatible** if absolute. A kernel must mutate heap, page tables, device MMIO, and task state. “Immutable OS” as marketing is a status inflation. Do not write it.

Hub: [overview.md](overview.md). Site SoT: [Advantages — Immutability](../overview/advantages.md#immutability). Pillars: [pillars.md](pillars.md). Threat model: [security.md](security.md).

Plan issues (open on 2026-09-11): [Track A #31](https://github.com/artofdream/ctos/issues/31) / in-repo [track-a.md](../04-roadmap/track-a.md), [Track B #40](https://github.com/artofdream/ctos/issues/40) / [track-b.md](../04-roadmap/track-b.md). A9: [#48](https://github.com/artofdream/ctos/issues/48). Tracks are subordinate to [principles.md](principles.md).

## Already practiced (probed)

These are **scoped** cuts on QEMU `virt`. They are not an immutable kernel.

| Scope | Probe | ADR |
| --- | --- | --- |
| Identity `.text`/`.rodata` RO+X; `.data`/heap RW+NX | `ro: nx data` / `ro: write fault` / `ro: ok` | [ADR-015](../03-adr/ADR-015-ro-nx-text-data.md) |
| `SCTLR.WXN` on | same RO+NX smoke | ADR-015 |
| Live identity `.text` torn after high-VA jump + vtable rewrite | `ident: reloc` / `ident: live` / `ident: ok` | [ADR-020](../03-adr/ADR-020-identity-fnptr-reloc.md) |

Heap, PTEs, UART/GIC, and coop stacks stay writable on purpose. `.rodata`/`.data`/heap identity tear is still **Planned**.

## Track A — RO app payloads, then A9 slot disconnect

The **goal** of this stance (sponsor clarification) is to **disconnect OS updates from apps**: a separate OS image vs app payloads. Update/replace the kernel without rebuilding apps, and the reverse. That is [A9 #48](https://github.com/artofdream/ctos/issues/48). It is **not** an “immutable OS” product sentence.

[Track A #31](https://github.com/artofdream/ctos/issues/31) must land first. A1 is a kernel SVC ABI mile ([syscall.md](syscall.md)). A2 is a `libctos` CRT mile ([ADR-022](../03-adr/ADR-022-libctos-crt.md)). A3 is a guest ELF64 `PT_LOAD` loader ([ADR-023](../03-adr/ADR-023-elf-pt-load-loader.md)). A4 is standing EL0 as normal mode for that loaded image ([ADR-024](../03-adr/ADR-024-standing-el0-normal.md)). **RO app payloads** are that loader mapping an image RO+X (R-only `PT_LOAD` already uses `map_el0_ro`; RX text stays Exec). A9 is **Planned after that ABI/loader**, not instead of it.

**Today:** one linked kernel ELF. No OS-image artifact, no app payload slot, no cross-update probe. **Not Verified.** Performance impact is the same honesty: expected costs and a future app-load CNTPCT gate, **no Verified delta** ([performance.md](performance.md#osapp-slot-disconnect-a9--expected-shape-not-a-bench)).

[Track B #40](https://github.com/artofdream/ctos/issues/40) must not use Linux-compat research to claim an immutable or container host. Containers stay a [non-goal](host-apps.md).

## Claim gate (ADR-style, no new ADR here)

Same rule as [ADR-001](../03-adr/ADR-001-honesty-harness-for-ctos.md) / [ADR-011](../03-adr/ADR-011-three-pillars.md):

1. Name the **scope** (which pages, which image).
2. Cite a **probe** (serial + `#[test_case]`, or source absence).
3. Only then **Verified**. Unprobed stays **Unknown**. Future work stays **Planned**.
4. Do not say “immutable OS,” “W^X kernel” as a product sentence, or “secure because RO.”

File presence of this note is not that probe. See the [honesty ledger](honesty-ledger.md).
