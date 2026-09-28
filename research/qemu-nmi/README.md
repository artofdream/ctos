# Research / pin: QEMU `TYPE_NMI` → async SError (ADR-083)

**Status (2026-09-19):** shippable **opt-in pin**. Proven on agent-box with
`CTOS_QEMU=tools/qemu-nmi/bin/qemu-system-aarch64` +
`CTOS_REQUIRE_TAKEN_SERROR=1` → serial `el0: serror` under EXPECT.
Stock distro QEMU still parks (`el0: serror-park`).

## Pin

| Field | Value |
| --- | --- |
| Upstream | QEMU **v10.0.0** (`7c949c53e936aa3a658d84ab53bae5cadaa5d59c`) |
| Patch | `0001-hw-arm-virt-TYPE_NMI-raise-SError.patch` |
| Prefix | `tools/qemu-nmi/` (bin + share; gitignored build output) |
| Build | `./scripts/build-qemu-nmi.sh` |

## Prove

```sh
./scripts/build-qemu-nmi.sh
export CTOS_QEMU="$PWD/tools/qemu-nmi/bin/qemu-system-aarch64"
export CTOS_REQUIRE_TAKEN_SERROR=1
./scripts/qemu-nmi-probe.py          # inject-nmi => ok
./scripts/qemu-smoke.sh              # requires el0: serror
```

Without the pin / without `CTOS_REQUIRE_TAKEN_SERROR=1`, smoke keeps requiring
`el0: serror-park` (ADR-053/081/082 stock path).

## Honesty

- Do **not** claim FEAT_NMI / FIQ / BRK is SError.
- The pin alone is not the umbrella claim. “EL0 isolated” is only the accepted [ADR-091](../../docs/03-adr/ADR-091-el0-isolated-accept-draft.md) sentence (M6); this job is one part of its taken-SError evidence class (ADR-090).
- Default GHA/Docker stay on distro QEMU unless opt-in build is enabled.
