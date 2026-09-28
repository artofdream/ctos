# Session memory — 2026-09-27 ADR-094 (G4)

- #143 got a plain commit replacing a local user path with `<user>`; #143 was merged forward into #144 with a merge commit (no force-push). ADR-094 is stacked on #144.
- EL0 execute-only (`AP[2:1]=00`, UXN clear) is EL1-RW. EL0 RO+X (`AP=11`) is EL1-RO too: write via the TTBR1 frame alias (`frame_cpu_va`).
- Once a page is EL0-accessible, PAN blocks EL1 cache maintenance by its VA. Wrap `dc`/`ic` by the EL0 VA in `pan::with_user_access`.
- The map-window L3 is shared by the kernel and user roots, so `user_el0_access` (user-root walk) sees the leaf the copy uses under kernel TTBR0.
- Kernel-built payloads used to stash strings on the execute-only code page. With the check they must live on an EL0-readable page: `with_el0_data` at `EL0_PAGE + 4096`.
- Leak build: an EL1 `copy_to_user` into an AP[2]=1 page takes an EL1 permission fault. So the probe skips write-ro once an earlier case leaked (`sys-ptr skip write-ro`).
- A `SYS_UART_WRITE` leak puts raw stub bytes on the same serial line as the next marker. Leave that smoke regex unanchored at `^`.
- Two QEMU runs in parallel collide on the QMP socket path in `qemu-serial-inject.py`. Run them serially.
- Don't edit `src/` while `qemu-smoke.sh` runs (it rebuilds leak variants mid-run).
