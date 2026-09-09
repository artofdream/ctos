#![no_std]
#![no_main]
#![cfg_attr(test, feature(custom_test_frameworks))]
#![cfg_attr(test, test_runner(crate::test_runner))]
#![cfg_attr(test, reexport_test_harness_main = "test_main")]

mod exception;
mod qemu;
mod uart;

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
