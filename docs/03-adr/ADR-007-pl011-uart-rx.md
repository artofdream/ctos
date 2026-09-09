# ADR-007 — PL011 UART RX as virt input

- Status: Accepted
- Date: 2026-09-09

## Context

[FR-08](../02-requirements/fr-nfr.md) already covers hardware interrupts (timer via the virt GIC, M5 / [ADR-006](ADR-006-gicv2-generic-timer.md)) and named “later input” as the rest of that ID. Roadmap M6 is that input path. The x86-era reading of FR-08 was a keyboard. Frozen IDs stay; the arm64 text is revised here.

Options on QEMU `virt`:

1. **PL011 UART RX** at `0x0900_0000` (already the console).
2. **virtio-input / virtio-keyboard** (virtio-mmio, virtqueues, DMA). No paging or heap yet (M7/M8). Too much device stack for one milestone.
3. **PL011 UARTCR.LBE loopback** as a self-test. QEMU **8.2** (Ubuntu 24.04 / this repo’s GHA and cloud probe) does not implement LBE — `hw/char/pl011.c` still says the loopback bit is unimplemented. LBE landed in later QEMU. A loopback-only probe would be a false pass on newer hosts and a false fail on 8.2.

Host stdin on `-serial stdio` *does* reach `pl011_receive` → the RX FIFO. `UARTLCR_H.FEN` toggles reset that FIFO, so a byte sent before `uart::init` is lost. Inject after `Hello World!` (init already ran).

## Decision

1. **M6 input is PL011 RX poll**, not virtio-keyboard and not a UART RX IRQ. FR-08’s “via the virt GIC” remains the M5 timer. Input does not take a new FR ID.
2. **Prove a received byte** with a host inject: `scripts/qemu-serial-inject.py` writes `0x41` (`'A'`) to QEMU stdin after it sees `Hello World!`. The hello kernel polls `UARTFR.RXFE` / `UARTDR` (DAIF.I still masked after the timer window) and prints `input: rx 0x41`.
3. **Fail closed.** Missing marker or `input: rx missed` fails `scripts/qemu-smoke.sh`. `#[test_case]` only asserts the FIFO is empty when cargo test does not inject — the character proof is the serial smoke.
4. **Do not claim** Raspberry Pi UART, virtio-input, GICv3 UART SPI, or QEMU LBE.

## Consequences

- `scripts/qemu-smoke.sh` needs `python3` to drive the inject (Dockerfile installs it).
- `cargo test` / `scripts/qemu-aarch64.sh` stay inject-free so M2–M5 cases do not wait on stdin.
- A later virtio-keyboard or UART-RX-via-GIC path needs a new ADR; it is not this milestone.
