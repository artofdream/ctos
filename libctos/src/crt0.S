// Minimal CRT: kernel ERET already set SP_EL0. Call main, then exit.
// Not crt0 from glibc/musl. No argv, no environ, no libc init.

    .section .text.boot, "ax"
    .align 2
    .global _start
    .type _start, %function
_start:
    bl main
    b ctos_exit
