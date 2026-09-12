# Session memory — 2026-09-12 (Track A / A4 standing EL0)

Fetched `origin/main` `cf65a6e` (A3 merge). Branch `cursor/a4-standing-el0-normal-1e14`. PR #54.

A4 is not a new loader. `install_task` vs `install_standing` (probe). `is_active()` true for both; `is_task()` only for the loaded image. `SYS_YIELD` / `SYS_UART_WRITE` print `el0: task-active` once. `SYS_EXIT` prints `el0: task-exit` / `el0: task-restored`. Unexpected lower-EL sync while a task is standing prints `el0: restore-fail` and returns to EL1 — does not park. Probe standing still parks on unexpected sync (A1–A3 unchanged).

Fault probe is `BR X0` to `MAP_WINDOW + 8*4096` (unmapped). Enter marker is `el0: task-enter` so smoke greps do not collide with `el0: task-ok`.

Local snapshot was stale at `aa46219`; had to fetch `origin/main` before branching. QEMU was not on PATH; installed `qemu-system-arm` 8.2.2.

Do not self-merge. Merger is `artofdream`.
