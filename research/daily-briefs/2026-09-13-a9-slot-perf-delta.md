# 2026-09-13 — A9 slot performance delta (ADR-046)

Shipped probe-only embed CNTPCT + `perf: slot-delta` on one boot. Production A9 stays FAT-only.

Serial (agent-box QEMU 10.0.13): `perf: app-load ticks=1079247` / `perf: embed-load ticks=62805` / `perf: slot-delta app=1079247 embed=62805`.

Still Planned (unchanged): PAN enable; taken SError; yank `_start`; umbrella EL0 isolation; product app hosting.
