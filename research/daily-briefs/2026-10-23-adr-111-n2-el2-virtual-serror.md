# 2026-10-23 — ADR-111 N2: EL2 shim → virtual SError on stock QEMU

- `tools/el2-shim` at PA 0 (`-device loader,cpu-num=0`), HCR_EL2 RW|AMO, `hvc #0xC7` sets VSE; ctos `el2-vserror` feature asks in the standing window.
- Full image: `el0: virtual serror taken (el2 vse) ec=0x2f` on stock 8.2.2 and 10.2.1. Negatives: no ask, A masked, no EL2 → park.
- Opt-in CI job; no claim change. See [ADR-111](../../docs/03-adr/ADR-111-n2-el2-shim-virtual-serror.md).
