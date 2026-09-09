# Session memory — 2026-09-09 (M6)

QEMU 8.2 `hw/char/pl011.c` does not implement UARTCR.LBE (`Need to implement the enable and loopback bits`). Do not use loopback as the M6 probe on Ubuntu 24.04 / GHA. Inject `0x41` on `-serial stdio` *after* `Hello World!` — `UARTLCR_H.FEN` toggle in `init` resets the RX FIFO, so a pre-init byte is lost. Poll `UARTFR.RXFE` / `UARTDR` with DAIF.I still masked. virtio-input needs virtqueues; skip until paging. Do not start M7. Do not claim Raspberry Pi.
