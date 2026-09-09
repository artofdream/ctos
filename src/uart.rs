//! PL011 UART on QEMU `virt` (MMIO base `0x0900_0000`).
//!
//! This is the early console. It is not a Raspberry Pi UART driver.

use core::fmt;
use spin::Mutex;

/// QEMU virt PL011. See QEMU `hw/arm/virt.c` / ARM DDI 0183.
const UART0_BASE: usize = 0x0900_0000;
const UART_DR: usize = 0x00;
const UART_FR: usize = 0x18;
const UART_IBRD: usize = 0x24;
const UART_FBRD: usize = 0x28;
const UART_LCR_H: usize = 0x2c;
const UART_CR: usize = 0x30;

const UART_FR_TXFF: u32 = 1 << 5;
const UART_CR_UARTEN: u32 = 1 << 0;
const UART_CR_TXE: u32 = 1 << 8;
const UART_CR_RXE: u32 = 1 << 9;
const UART_LCR_H_FEN: u32 = 1 << 4;
const UART_LCR_H_WLEN_8: u32 = 0b11 << 5;

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
}

impl fmt::Write for Pl011 {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
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
