#![no_std]
#![no_main]
#![feature(alloc_error_handler)]
#![cfg_attr(test, feature(custom_test_frameworks))]
#![cfg_attr(test, test_runner(crate::test_runner))]
#![cfg_attr(test, reexport_test_harness_main = "test_main")]

extern crate alloc;

mod el0;
mod exception;
mod frame;
mod gic;
mod heap;
mod paging;
mod perf;
mod qemu;
mod sched;
mod timer;
mod uart;
mod wx;

use core::arch::global_asm;
use core::panic::PanicInfo;

global_asm!(
    r#"
    .section .text.boot, "ax"
    .global _start
    _start:
        ldr x0, =__stack_top
        mov sp, x0

        ldr x0, =__bss_start
        ldr x1, =__bss_end
    1:
        cmp x0, x1
        b.ge 2f
        str xzr, [x0], #8
        b 1b
    2:
        bl kernel_main
    3:
        wfe
        b 3b
    "#
);

#[no_mangle]
pub extern "C" fn kernel_main() -> ! {
    uart::UART.lock().init();
    exception::init();
    frame::init();
    paging::init();
    heap::init();
    sched::init();
    gic::init();
    timer::init();
    println!("Hello World!");

    #[cfg(feature = "force-fail")]
    panic!("force-fail");

    #[cfg(all(test, not(feature = "force-fail")))]
    {
        test_main();
        qemu::exit_success();
    }

    #[cfg(not(any(test, feature = "force-fail")))]
    {
        // Serial proof for qemu-smoke (FR-09 / M7): MMU on + map/unmap.
        if !paging::observe_probe() {
            uart::write_str_raw("paging: probe missed\n");
        }
        // Serial proof for qemu-smoke (FR-10 / M8): Box + Vec on the heap.
        if !heap::observe_probe() {
            uart::write_str_raw("heap: probe missed\n");
        }
        // Serial proof for qemu-smoke (FR-11 / M9): two cooperative tasks.
        if !sched::observe_probe() {
            uart::write_str_raw("sched: probe missed\n");
        }
        // Serial proof for qemu-smoke (NFR-10 / ADR-012): heap PXN + IABORT.
        if !wx::observe_probe() {
            uart::write_str_raw("wx: probe missed\n");
        }
        // Serial proof for qemu-smoke (NFR-07 / ADR-011): CNTPCT advances.
        if !perf::observe_probe() {
            uart::write_str_raw("perf: probe missed\n");
        }
        // Serial proof for qemu-smoke (FR-08): several CNTP ticks, then remask
        // so the M3/M4 probes are not interrupted. First tick still prints
        // `timer: tick`. Samples feed the IRQ-to-handler CNTPCT probe.
        if !timer::observe_ticks(8) {
            uart::write_str_raw("timer: tick missed\n");
        }
        if !perf::observe_irq_delta() {
            uart::write_str_raw("perf: irq-delta missed\n");
        }
        // Serial proof for qemu-smoke (FR-08 input / M6): host-injected RX.
        // QEMU 8.2 has no PL011 LBE; scripts/qemu-smoke.sh writes PROBE_BYTE
        // after Hello World! DAIF.I stays masked (timer already remasked).
        if !uart::observe_probe_byte() {
            uart::write_str_raw("input: rx missed\n");
        }
        // Serial proof for qemu-smoke (FR-06): handler must print and return.
        exception::breakpoint();
        // FR-07: near-empty thread SP + nested BRK → fatal stack + marker.
        exception::trigger_fatal_nested();
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    #[cfg(any(test, feature = "force-fail"))]
    qemu::exit_failure();
    #[cfg(not(any(test, feature = "force-fail")))]
    loop {
        unsafe {
            core::arch::asm!("wfe", options(nomem, nostack));
        }
    }
}

#[alloc_error_handler]
fn alloc_error(_layout: core::alloc::Layout) -> ! {
    // Do not format — formatting an OOM could allocate again.
    uart::write_str_raw("heap: oom\n");
    #[cfg(any(test, feature = "force-fail"))]
    qemu::exit_failure();
    #[cfg(not(any(test, feature = "force-fail")))]
    loop {
        unsafe {
            core::arch::asm!("wfe", options(nomem, nostack));
        }
    }
}

#[cfg(test)]
pub(crate) trait Testable {
    fn run(&self);
}

#[cfg(test)]
impl<T: Fn()> Testable for T {
    fn run(&self) {
        print!("{}...\t", core::any::type_name::<T>());
        self();
        println!("[ok]");
    }
}

#[cfg(test)]
fn test_runner(tests: &[&dyn Testable]) {
    println!("Running {} tests", tests.len());
    for test in tests {
        test.run();
    }
    qemu::exit_success();
}

#[cfg(test)]
#[test_case]
fn trivial_eq() {
    assert_eq!(1 + 1, 2);
}

#[cfg(test)]
#[test_case]
fn println_once() {
    println!("test println");
}
