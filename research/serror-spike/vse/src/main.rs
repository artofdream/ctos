//! ADR-108 spike: stock-QEMU *virtual* SError (HCR_EL2.VSE) taken at EL1
//! from EL0. Stand-alone image, not the ctos kernel. Boots at EL2
//! (`-M virt,virtualization=on`), sets HCR_EL2, ERETs straight to EL0.
//! Mode byte at 0x4020_0000 (QEMU `-device loader`):
//!   0 = AMO+VSE, EL0 A clear  -> expect taken (vector 0x580, EC 0x2f)
//!   1 = AMO only (no VSE)     -> expect none (EL0 reaches svc)
//!   2 = AMO+VSE, EL0 A set    -> expect none (masked)
//!   3 = VSE without AMO       -> expect none (not routed)
//!   4 = HCR RW only, EL0 A clear, spin  -> host-injected physical SError
//!       (pinned QEMU `nmi`) should be taken at EL1 vec 0x580
//!   5 = same, EL0 A set (masked)       -> expect nothing printed after banner
//!   6 = HCR RW only, EL0 loads unassigned PA 4 GiB (bad device access)
//!       -> QEMU reports a *synchronous* external abort, not an SError
//! Mode | 0x10: cpu0 starts cpu1 via PSCI CPU_ON (smc) and parks; cpu1 runs
//! the low nibble (`-smp 2`).
#![no_std]
#![no_main]
use core::arch::global_asm;

global_asm!(r#"
.section .text.boot,"ax"
.global _entry
_entry:
    mrs x0, CurrentEL
    lsr x0, x0, #2
    cmp x0, #2
    b.ne not_el2
    ldr x1, =0x40200000
    ldrb w19, [x1]
    mrs x4, mpidr_el1
    and x4, x4, #0xff
    cbnz x4, 8f
    tbz w19, #4, 8f
    // mode | 0x10: cpu0 starts cpu1 at _entry (PSCI CPU_ON, smc), then parks
    ldr x0, =0xc4000003
    mov x1, #1
    adr x2, _entry
    mov x3, #0
    smc #0
9:  wfe
    b 9b
8:  and w19, w19, #0xf
    adr x1, vec1
    msr vbar_el1, x1
    ldr x1, =stack1_top
    msr sp_el1, x1
    mov x1, sp
    ldr x1, =stack1_top
    mov sp, x1
    mov w0, w19
    mov x1, x4
    bl rust_banner
    mov x2, #(1 << 31)
    cmp w19, #4
    b.hs 2f
    cmp w19, #3
    b.eq 1f
    orr x2, x2, #(1 << 5)
1:  cmp w19, #1
    b.eq 2f
    orr x2, x2, #(1 << 8)
2:  msr hcr_el2, x2
    ldr x1, =0x30d00800
    msr sctlr_el1, x1
    mov x3, #0
    cmp w19, #5
    b.eq 10f
    cmp w19, #2
    b.ne 3f
10:
    mov x3, #(1 << 8)
3:  msr spsr_el2, x3
    adr x1, el0_code
    cmp w19, #6
    b.ne 12f
    adr x1, el0_badpa
    b 11f
12: cmp w19, #4
    b.lo 11f
    adr x1, el0_spin
11: msr elr_el2, x1
    isb
    eret
not_el2:
    bl rust_not_el2
4:  wfe
    b 4b

.balign 64
el0_code:
    mov x9, #20000
5:  subs x9, x9, #1
    b.ne 5b
    svc #0
6:  b 6b
el0_spin:
    b el0_spin
el0_badpa:
    mov x0, #0x100000000
    ldr x1, [x0]
    svc #0
13: b 13b

.macro VENT off
.balign 0x80
    mov x0, #\off
    b vcommon
.endm
.balign 2048
vec1:
VENT 0x000
VENT 0x080
VENT 0x100
VENT 0x180
VENT 0x200
VENT 0x280
VENT 0x300
VENT 0x380
VENT 0x400
VENT 0x480
VENT 0x500
VENT 0x580
VENT 0x600
VENT 0x680
VENT 0x700
VENT 0x780
vcommon:
    mrs x1, esr_el1
    mrs x2, spsr_el1
    mrs x3, mpidr_el1
    and x3, x3, #0xff
    bl rust_vec
7:  wfe
    b 7b
"#);

const UART: *mut u32 = 0x0900_0000 as *mut u32;

fn putc(c: u8) {
    unsafe { core::ptr::write_volatile(UART, c as u32) }
}
fn puts(s: &str) {
    for b in s.bytes() {
        putc(b);
    }
}
fn hex(v: u64) {
    puts("0x");
    let mut started = false;
    for i in (0..16).rev() {
        let n = ((v >> (i * 4)) & 0xf) as u8;
        if n != 0 || started || i == 0 {
            started = true;
            putc(if n < 10 { b'0' + n } else { b'a' + n - 10 });
        }
    }
}

#[no_mangle]
extern "C" fn rust_banner(mode: u32, cpu: u64) {
    puts("vse: el2 mode=");
    hex(mode as u64);
    puts(" cpu=");
    hex(cpu);
    puts("\n");
    if mode == 4 || mode == 5 {
        puts("vse: el0 spin (inject now)\n");
    }
}

#[no_mangle]
extern "C" fn rust_not_el2() {
    puts("vse: not-el2 (need -M virt,virtualization=on)\n");
}

#[no_mangle]
extern "C" fn rust_vec(off: u64, esr: u64, spsr: u64, cpu: u64) {
    puts("vse: cpu=");
    hex(cpu);
    puts(" ");
    let ec = esr >> 26;
    if off == 0x580 && ec == 0x2f {
        puts("serror taken vec=");
        hex(off);
        puts(" from=el0 ec=");
        hex(ec);
        puts(" esr=");
        hex(esr);
        puts(" spsr=");
        hex(spsr);
        puts("\n");
    } else if off == 0x400 && ec == 0x15 {
        puts("none (el0 reached svc) spsr=");
        hex(spsr);
        puts("\n");
    } else if off == 0x400 && (ec == 0x24 || ec == 0x20) {
        puts("sync abort (not serror) vec=");
        hex(off);
        puts(" ec=");
        hex(ec);
        puts(" dfsc=");
        hex(esr & 0x3f);
        puts("\n");
    } else {
        puts("unexpected vec=");
        hex(off);
        puts(" esr=");
        hex(esr);
        puts("\n");
    }
    puts("vse: end\n");
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
