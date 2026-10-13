# 2026-10-13 — ADR-100: forward-only host / user name scrub

- **Ask:** sponsor decision 2026-10-13 20:49 CEST (DSO relay): scrub the remaining host and user name mentions left after ADR-093, forward only (no history rewrite), and make the guard fail closed for those classes. $0.
- **Done:** two workstation names → “amd64 workstation” / “arm64 workstation”; agent VM, EC2 private and GitHub runner host names and one machine ID → placeholders. Two scripts stop printing host names. Guard: 8 new classes (machine names by SHA-256), self-test `classes=15`. Counts by class in [ADR-100](../../docs/03-adr/ADR-100-host-user-name-scrub-guard.md).
- **Also (task 2):** repo variable `CTOS_BUILD_QEMU_NMI` deleted after grepping workflows (no reference). Ledger row.
- **Not claimed:** history is unchanged; PR bodies and CI logs are not scanned.
