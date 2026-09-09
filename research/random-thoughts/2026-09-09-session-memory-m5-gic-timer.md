# Session memory — 2026-09-09 (M5)

QEMU 8.2 `-machine virt` default is GICv2 (`0x08000000` dist / `0x08010000` CPU). Do not program GICv3 redistributors against that machine line. EL1 physical timer is CNTP_*_EL0, PPI 30. IRQ slot is current-EL SP_EL0 bank (`0x080`) because SPSel=0 after M4. DAIF.I stays set except the observe window so BRK/fatal are not interrupted. IRQ path uses raw UART. First tick prints `timer: tick`. If dropping EL2→EL1, set CNTHCTL_EL2 EL1PCTEN|EL1PCEN. Do not start M6 input. Do not claim Raspberry Pi.
