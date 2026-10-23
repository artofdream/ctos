# Taken-SError QEMU spike (ADR-109)

Stand-alone aarch64 image (`vse/`, **not** the ctos kernel) plus a fail-closed matrix.

```sh
# build (nightly, aarch64-unknown-none, rust-src)
(cd vse && cargo build --release)
E=vse/target/aarch64-unknown-none/release/vse-spike
# stock QEMU (any of 8.2.2 / 10.2.1 / master): inject rows must be refused
./serror-matrix.py $E
# pinned B1 QEMU (../../scripts/build-qemu-nmi.sh)
CTOS_QEMU=../../tools/qemu-nmi/bin/qemu-system-aarch64 CTOS_SPIKE_PIN=1 ./serror-matrix.py $E
# gdbstub route (same HMP nmi)
CTOS_QEMU=... ./gdb-nmi.sh $E
```

Boot: `-M virt,virtualization=on -cpu cortex-a76 [-smp 2] -kernel $E -device loader,addr=0x40200000,data=<mode>,data-len=1`.

| Mode | Setup | Expect |
| --- | --- | --- |
| 0 | `HCR_EL2` RW+AMO+VSE, ERET to EL0 (A clear) | `serror taken vec=0x580 from=el0 ec=0x2f` |
| 1 | RW+AMO, no VSE | `none (el0 reached svc)` |
| 2 | RW+AMO+VSE, EL0 A set | `none` (masked) |
| 3 | RW+VSE, no AMO | `none` (not routed) |
| 4 | RW only, EL0 spins | host `nmi`: pinned → taken; stock → refused |
| 5 | as 4, EL0 A set | pinned `nmi` → nothing taken |
| 6 | RW only, EL0 loads PA 4 GiB | sync abort vec 0x400 EC 0x24 DFSC 0x10 (not SError) |
| +0x10 | cpu0 starts cpu1 with PSCI `CPU_ON` (smc) and parks; cpu1 runs the mode (`-smp 2`) | as above, `cpu=0x1` |

The matrix exits 1 on any mismatch. Log of the 2026-10-23 run: `2026-10-23-matrix-log.txt` (52/52 ok over 4 QEMUs).
