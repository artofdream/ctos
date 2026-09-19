# ADR-083 brief — pinned QEMU TYPE_NMI → el0: serror

Date: 2026-09-19 CEST. Host: agent-box (EVO-X2 intent; CloudAgent HELD).

## Result

- Built QEMU **v10.0.0** `7c949c53` + local patch → `tools/qemu-nmi/bin/qemu-system-aarch64`.
- `inject-nmi` returns **ok** (probe matrix).
- Smoke with `CTOS_REQUIRE_TAKEN_SERROR=1` prints bare **`el0: serror`** (QMP stop/inject/cont on arm cue).
- Stock 10.0.13 still errors + `el0: serror-park`.

## Not claimed

FEAT_NMI≠SError; umbrella EL0 isolated; default CI Verified taken (opt-in only).
