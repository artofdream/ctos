# 2026-10-23 — ADR-109 taken-SError QEMU spike

- Go: sponsor, relayed by DSO 19:25 CEST. Amd64 workstation only (TCG); builds in an ubuntu:24.04 container.
- Stock QEMU 8.2.2 / 10.2.1 / master 11.1.50 refuse `nmi`; the pinned B1 build injects it, and can target cpu1 (`cpu-index=1`).
- Virtual SError (`HCR_EL2.VSE`, guest at EL2) is taken from EL0 at vector 0x580 (EC 0x2f) on every QEMU, on cpu0 and cpu1.
- Bad-PA access = sync external abort (EC 0x24), not SError. gdbstub `monitor nmi` = the same HMP nmi.
- Matrix 52/52 ok with negatives. Stand-alone image only; no claim change. See [ADR-109](../../docs/03-adr/ADR-109-taken-serror-qemu-spike.md).
