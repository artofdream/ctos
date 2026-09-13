//! GICv2 on QEMU `virt` (FR-08 / M5).
//!
//! `-machine virt` (QEMU 8.2) defaults to GICv2. Distributor `0x0800_0000`,
//! CPU interface `0x0801_0000`. This is not a GICv3 redistributor driver
//! and not a Raspberry Pi interrupt controller. See ADR-006.

use crate::timer;
use crate::uart;

/// QEMU virt GICv2 distributor (see QEMU `hw/arm/virt.c` / `virt.h`).
const GICD_BASE: usize = 0x0800_0000;
/// QEMU virt GICv2 CPU interface.
const GICC_BASE: usize = 0x0801_0000;

const GICD_CTLR: usize = 0x000;
const GICD_TYPER: usize = 0x004;
const GICD_ISENABLER: usize = 0x100;
const GICD_IPRIORITYR: usize = 0x400;

const GICC_CTLR: usize = 0x000;
const GICC_PMR: usize = 0x004;
const GICC_IAR: usize = 0x00c;
const GICC_EOIR: usize = 0x010;

const GICD_CTLR_ENABLE: u32 = 1;
const GICC_CTLR_ENABLE: u32 = 1;
const GICC_PMR_ALLOW_ALL: u32 = 0xff;
const PPI_PRIORITY: u8 = 0xa0;
const SPURIOUS_ID: u32 = 1023;

unsafe fn write32(base: usize, offset: usize, value: u32) {
    core::ptr::write_volatile((base + offset) as *mut u32, value);
}

unsafe fn read32(base: usize, offset: usize) -> u32 {
    core::ptr::read_volatile((base + offset) as *const u32)
}

unsafe fn write8(base: usize, offset: usize, value: u8) {
    core::ptr::write_volatile((base + offset) as *mut u8, value);
}

fn dsb_ish() {
    unsafe {
        core::arch::asm!("dsb ish", options(nostack, preserves_flags));
    }
}

/// Enable Group-0 forwarding on the distributor and CPU interface.
///
/// Default: Group 0 signals as **IRQ** (`GICC_CTLR.FIQEn` clear). ADR-043
/// temporarily sets FIQEn for the standing lower-EL FIQ probe only.
pub fn init() {
    unsafe {
        write32(GICD_BASE, GICD_CTLR, 0);
        write32(GICC_BASE, GICC_CTLR, 0);
        write32(GICC_BASE, GICC_PMR, GICC_PMR_ALLOW_ALL);
        write32(GICD_BASE, GICD_CTLR, GICD_CTLR_ENABLE);
        write32(GICC_BASE, GICC_CTLR, GICC_CTLR_ENABLE);
    }
    dsb_ish();
}

/// `GICC_CTLR.FIQEn` — when set, Group 0 interrupts signal as FIQ (GICv2).
const GICC_CTLR_FIQEN: u32 = 1 << 3;

/// Route Group-0 interrupts as FIQ for the ADR-043 standing probe.
/// Caller must restore via [`route_group0_as_irq`] so the IRQ path stays live.
pub fn route_group0_as_fiq() {
    unsafe {
        let ctlr = read32(GICC_BASE, GICC_CTLR);
        write32(GICC_BASE, GICC_CTLR, ctlr | GICC_CTLR_FIQEN);
    }
    dsb_ish();
}

/// Restore Group-0 → IRQ (clear FIQEn). Default after [`init`].
pub fn route_group0_as_irq() {
    unsafe {
        let ctlr = read32(GICC_BASE, GICC_CTLR);
        write32(GICC_BASE, GICC_CTLR, ctlr & !GICC_CTLR_FIQEN);
    }
    dsb_ish();
}

/// Same ack/EOI path as IRQ: Group 0 uses `GICC_IAR` / `GICC_EOIR`.
pub fn handle_fiq() {
    handle_irq();
}

/// Enable one PPI (16..=31) at a mid priority. PPIs are CPU-private.
pub fn enable_ppi(id: u32) {
    assert!((16..32).contains(&id), "not a PPI");
    unsafe {
        write8(GICD_BASE, GICD_IPRIORITYR + id as usize, PPI_PRIORITY);
        write32(GICD_BASE, GICD_ISENABLER, 1 << id);
    }
    dsb_ish();
}

pub fn ack() -> u32 {
    unsafe { read32(GICC_BASE, GICC_IAR) }
}

pub fn eoi(iar: u32) {
    unsafe {
        write32(GICC_BASE, GICC_EOIR, iar);
    }
    dsb_ish();
}

pub fn mask_irqs() {
    unsafe {
        core::arch::asm!("msr daifset, #2", "isb", options(nostack, preserves_flags));
    }
}

pub fn unmask_irqs() {
    unsafe {
        core::arch::asm!("msr daifclr, #2", "isb", options(nostack, preserves_flags));
    }
}

/// Acknowledge, dispatch the timer PPI, then EOI. Uses raw UART only.
pub fn handle_irq() {
    let iar = ack();
    let id = iar & 0x3ff;
    if id == timer::PPI {
        timer::on_interrupt();
    } else if id <= SPURIOUS_ID {
        uart::write_str_raw("irq: unexpected\n");
    }
    if id < 1020 {
        eoi(iar);
    }
}

#[allow(dead_code)] // `#[test_case]` read-back.
pub fn typer() -> u32 {
    unsafe { read32(GICD_BASE, GICD_TYPER) }
}

#[allow(dead_code)] // `#[test_case]` read-back.
pub fn cpu_enabled() -> bool {
    unsafe { read32(GICC_BASE, GICC_CTLR) & GICC_CTLR_ENABLE != 0 }
}

#[cfg(test)]
#[test_case]
fn gicd_typer_readable() {
    // GICv2 TYPER is non-zero on virt (CPU-number field at least).
    assert_ne!(typer(), 0);
    assert!(cpu_enabled());
}
