#![no_std]
#![no_main]

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
    println!("Hello World!");

    loop {
        unsafe {
            core::arch::asm!("wfe", options(nomem, nostack));
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {
        unsafe {
            core::arch::asm!("wfe", options(nomem, nostack));
        }
    }
}
