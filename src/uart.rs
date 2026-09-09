//! PL011 UART on QEMU `virt` (MMIO base `0x0900_0000`).
//!
//! Early console (TX) and M6 input (RX). QEMU 8.2’s PL011 does **not**
//! implement UARTCR.LBE loopback (`/* ??? Need to implement the enable
//! and loopback bits. */` in `hw/char/pl011.c`). The fail-closed RX
//! probe is a host-injected byte on `-serial stdio` (ADR-007).
//! This is not a Raspberry Pi UART driver.

use core::fmt;
use core::fmt::Write;
use spin::Mutex;

/// QEMU virt PL011. See QEMU `hw/arm/virt.c` / ARM DDI 0183.
const UART0_BASE: usize = 0x0900_0000;
const UART_DR: usize = 0x00;
const UART_FR: usize = 0x18;
const UART_IBRD: usize = 0x24;
const UART_FBRD: usize = 0x28;
const UART_LCR_H: usize = 0x2c;
const UART_CR: usize = 0x30;

const UART_FR_RXFE: u32 = 1 << 4;
const UART_FR_TXFF: u32 = 1 << 5;
const UART_CR_UARTEN: u32 = 1 << 0;
const UART_CR_TXE: u32 = 1 << 8;
const UART_CR_RXE: u32 = 1 << 9;
const UART_LCR_H_FEN: u32 = 1 << 4;
const UART_LCR_H_WLEN_8: u32 = 0b11 << 5;

/// Byte `scripts/qemu-smoke.sh` injects after `Hello World!` (ASCII `'A'`).
#[allow(dead_code)] // hello kernel only; cargo test does not inject.
pub const PROBE_BYTE: u8 = 0x41;

/// Wait this many counter seconds before `observe_rx` gives up.
const OBSERVE_TIMEOUT_SECS: u64 = 2;

pub static UART: Mutex<Pl011> = Mutex::new(Pl011 { base: UART0_BASE });

pub struct Pl011 {
    base: usize,
}

impl Pl011 {
    unsafe fn write_reg(&self, offset: usize, value: u32) {
        core::ptr::write_volatile((self.base + offset) as *mut u32, value);
    }

    unsafe fn read_reg(&self, offset: usize) -> u32 {
        core::ptr::read_volatile((self.base + offset) as *const u32)
    }

    /// Minimal TX enable. QEMU virt usually accepts DR writes without this;
    /// the sequence is cheap and matches a real PL011 bring-up.
    pub fn init(&mut self) {
        unsafe {
            self.write_reg(UART_CR, 0);
            // 115200-ish at a 24 MHz reference; QEMU ignores baud for stdio.
            self.write_reg(UART_IBRD, 13);
            self.write_reg(UART_FBRD, 1);
            self.write_reg(UART_LCR_H, UART_LCR_H_WLEN_8 | UART_LCR_H_FEN);
            self.write_reg(UART_CR, UART_CR_UARTEN | UART_CR_TXE | UART_CR_RXE);
        }
    }

    pub fn write_byte(&mut self, byte: u8) {
        unsafe {
            while self.read_reg(UART_FR) & UART_FR_TXFF != 0 {
                core::hint::spin_loop();
            }
            self.write_reg(UART_DR, byte as u32);
        }
    }

    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            if byte == b'\n' {
                self.write_byte(b'\r');
            }
            self.write_byte(byte);
        }
    }

    /// `UARTFR.RXFE`: receive FIFO empty.
    pub fn rx_empty(&self) -> bool {
        unsafe { self.read_reg(UART_FR) & UART_FR_RXFE != 0 }
    }

    /// One byte from UARTDR, or `None` if the RX FIFO is empty.
    pub fn try_recv(&self) -> Option<u8> {
        if self.rx_empty() {
            None
        } else {
            Some(unsafe { (self.read_reg(UART_DR) & 0xff) as u8 })
        }
    }
}

fn cntfrq() -> u64 {
    let freq: u64;
    unsafe {
        core::arch::asm!("mrs {f}, cntfrq_el0", f = out(reg) freq);
    }
    freq
}

fn cntpct() -> u64 {
    let t: u64;
    unsafe {
        core::arch::asm!("mrs {t}, cntpct_el0", t = out(reg) t);
    }
    t
}

/// Poll RX until a byte arrives or the physical counter times out.
///
/// Does not unmask IRQs. Do not `wfi` — there is no UART IRQ in M6.
#[allow(dead_code)] // hello kernel only; cargo test does not inject.
pub fn observe_rx() -> Option<u8> {
    let u = raw();
    let freq = cntfrq();
    let timeout = if freq == 0 {
        125_000_000
    } else {
        freq.saturating_mul(OBSERVE_TIMEOUT_SECS)
    };
    let t0 = cntpct();
    loop {
        if let Some(b) = u.try_recv() {
            return Some(b);
        }
        if cntpct().wrapping_sub(t0) > timeout {
            return None;
        }
        core::hint::spin_loop();
    }
}

/// Serial marker for qemu-smoke. Returns whether `PROBE_BYTE` arrived.
#[allow(dead_code)] // hello kernel only; cargo test does not inject.
pub fn observe_probe_byte() -> bool {
    match observe_rx() {
        Some(b) if b == PROBE_BYTE => {
            let mut w = raw();
            let _ = writeln!(w, "input: rx {b:#04x}");
            true
        }
        Some(b) => {
            let mut w = raw();
            let _ = writeln!(w, "input: rx {b:#04x} (unexpected)");
            false
        }
        None => false,
    }
}

impl fmt::Write for Pl011 {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

/// Direct PL011, no `spin::Mutex`. Fatal / nested paths must not wait on
/// a lock the interrupted context may already hold (ADR-004 / ADR-005).
pub fn raw() -> Pl011 {
    Pl011 { base: UART0_BASE }
}

pub fn write_str_raw(s: &str) {
    raw().write_string(s);
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::uart::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    UART.lock().write_fmt(args).unwrap();
}

#[cfg(test)]
#[test_case]
fn uart_rx_fifo_empty_without_host_byte() {
    // cargo test does not inject on stdin. An empty FIFO is the honest
    // register probe; the received-character proof is qemu-smoke.
    assert!(raw().rx_empty());
    assert_eq!(raw().try_recv(), None);
}
