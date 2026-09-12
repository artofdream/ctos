#![no_std]
#![no_main]
#![feature(alloc_error_handler)]
#![cfg_attr(test, feature(custom_test_frameworks))]
#![cfg_attr(test, test_runner(crate::test_runner))]
#![cfg_attr(test, reexport_test_harness_main = "test_main")]

extern crate alloc;

mod asid;
mod el0;
mod exception;
mod fat;
mod frame;
mod gic;
mod guard;
mod heap;
mod libctos;
mod loader;
mod paging;
mod pan;
mod perf;
mod qemu;
mod ro;
mod sched;
mod syscall;
mod teardown;
mod timer;
mod ttbr1;
mod uart;
mod vfs;
mod virtio;
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
    paging::init();
    // After MMU + high VBAR: fetch the rest from the TTBR1 alias
    // (ADR-019). ADR-020 rewrites rustc vtables then unmaps live
    // identity `.text`. ADR-025 unmaps identity `.rodata`. `_start`
    // stays at 0x40080000. Not “the kernel moved.”
    paging::jump_high(kernel_main_high as *const () as usize as u64);
}

#[inline(never)]
#[no_mangle]
extern "C" fn kernel_main_high() -> ! {
    uart::write_str_raw("ident: jump\n");
    if !paging::rewrite_identity_fn_ptrs() {
        uart::write_str_raw("ident: reloc missed\n");
    }
    if !paging::tear_identity_text_range() {
        uart::write_str_raw("ident: range missed\n");
    }
    if !paging::tear_live_identity_text() {
        uart::write_str_raw("ident: live missed\n");
    }
    if !paging::rewrite_identity_rodata_ptrs() {
        uart::write_str_raw("ident: ro-reloc missed\n");
    }
    if !paging::tear_identity_rodata() {
        uart::write_str_raw("ident: rodata missed\n");
    }
    // ADR-026: read ID_AA64MMFR1_EL1.PAN. Does not MSR PAN.
    // Prints `pan: id=` / `pan: absent` on `-cpu cortex-a57`.
    if !pan::observe_probe() {
        uart::write_str_raw("pan: probe missed\n");
    }
    // After MMU + D-cache (SCTLR.C). A pre-MMU store to .bss can be
    // invisible to later cached reads (PR #20 test image; cts-ai Docker
    // hello lost `frame::ALLOC` / `USER_MAP_OK` the same way).
    perf::mark_early();
    frame::init();
    heap::init();
    vfs::init();
    virtio::init();
    fat::init();
    sched::init();
    gic::init();
    timer::init();
    println!("Hello World!");
    let _ = perf::mark_ready();

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
        // Serial proof for qemu-smoke (NFR-10 / ADR-014): linker-stack guards.
        if !guard::observe_probe() {
            uart::write_str_raw("guard: probe missed\n");
        }
        // Serial proof for qemu-smoke (NFR-10 / ADR-015): RO+NX text/data.
        if !ro::observe_probe() {
            uart::write_str_raw("ro: probe missed\n");
        }
        // Serial proof for qemu-smoke (NFR-10 / ADR-013): EL0 first mile + read.
        if !el0::observe_probe() {
            uart::write_str_raw("el0: probe missed\n");
        }
        // Serial proof for qemu-smoke (Track A / A1 / ADR-021): SVC ABI.
        // Not app hosting. Not a Linux ABI.
        if !syscall::observe_probe() {
            uart::write_str_raw("svc: probe missed\n");
        }
        // Serial proof for qemu-smoke (Track A / A2 / ADR-022): libctos CRT.
        // Not an ELF loader. Not app hosting.
        if !libctos::observe_probe() {
            uart::write_str_raw("libctos: probe missed\n");
        }
        // Serial proof for qemu-smoke (Track A / A3 / ADR-023): guest ELF
        // PT_LOAD into user TTBR0. Not a Linux ABI. Not app hosting.
        if !loader::observe_probe() {
            uart::write_str_raw("loader: probe missed\n");
        }
        // Serial proof for qemu-smoke (Track A / A4 / ADR-024): standing
        // EL0 as normal mode for a loaded image until exit. Fail-closed
        // restore. Not isolation. Not app hosting.
        if !el0::observe_standing_task() {
            uart::write_str_raw("el0: task missed\n");
        }
        // Serial proof for qemu-smoke (Track A / A6 / ADR-027): thin VFS
        // + memfs create/write/read/close. Not FAT. Not app hosting.
        if !vfs::observe_probe() {
            uart::write_str_raw("fs: probe missed\n");
        }
        // Serial proof for qemu-smoke (Track A / A7 / ADR-028): virtio-blk
        // sector R/W. Host `-drive` without this guest path is not a probe.
        if !virtio::observe_probe() {
            uart::write_str_raw("blk: probe missed\n");
        }
        // Serial proof for qemu-smoke (Track A / A7 / ADR-028): FAT16
        // `/probe` through the same VFS `open`. Not a second open story.
        if !fat::observe_probe() {
            uart::write_str_raw("fat: probe missed\n");
        }
        // Serial proof for qemu-smoke (NFR-10 / ADR-013): ASID isolation mile.
        if !asid::observe_probe() {
            uart::write_str_raw("asid: probe missed\n");
        }
        // Serial proof for qemu-smoke (NFR-10 / ADR-016 + ADR-017):
        // TTBR1 private page + EL1 high-VA fetch.
        if !ttbr1::observe_probe() {
            uart::write_str_raw("ttbr1: probe missed\n");
        }
        // Serial proof for qemu-smoke (NFR-10 / ADR-018 + ADR-019 + ADR-020
        // + ADR-025): split tables + 16 KiB dedicated range + high jump +
        // vtable reloc + live identity `.text` tear + identity `.rodata`
        // tear. `.data` / heap stay. Not “the kernel moved.”
        if !teardown::observe_probe() {
            uart::write_str_raw("ident: probe missed\n");
        }
        // PAN ID was printed in kernel_main_high (ADR-026). Fail-closed
        // if that cut did not publish `pan: absent`.
        if !pan::pan_absent_ready() {
            uart::write_str_raw("pan: probe missed\n");
        }
        // Serial proof for qemu-smoke (NFR-07 / ADR-011): CNTPCT advances.
        if !perf::observe_probe() {
            uart::write_str_raw("perf: probe missed\n");
        }
        // Serial proof for qemu-smoke (NFR-08): boot-to-ready CNTPCT.
        if !perf::observe_boot_delta() {
            uart::write_str_raw("perf: boot-delta missed\n");
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
    #[allow(dead_code)] // invoked via high vtable (ADR-019), not `test.run()`.
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

/// rustc `dyn` vtable: drop, size, align, then methods. After ADR-019
/// the method address is an identity VA — BLR it through the high alias.
#[cfg(test)]
#[repr(C)]
struct DynFat {
    data: *const (),
    vtable: *const usize,
}

#[cfg(test)]
fn call_testable_run_high(test: &dyn Testable) {
    let fat: DynFat = unsafe { core::mem::transmute_copy(&(test as *const dyn Testable)) };
    let run = unsafe { *fat.vtable.add(3) } as u64;
    let f: fn(*const ()) = unsafe { core::mem::transmute(paging::to_high_va(run)) };
    f(fat.data);
}

#[cfg(test)]
fn test_runner(tests: &[&dyn Testable]) {
    println!("Running {} tests", tests.len());
    for test in tests {
        call_testable_run_high(*test);
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
