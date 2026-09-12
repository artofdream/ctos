// Fallback hello if the Rust payload is too large for one EL0 page.
// Calls the same libctos stubs (ctos_uart_write / ctos_yield / ctos_exit).

    .text
    .align 2
    .global main
    .type main, %function
main:
    adr x0, msg_hi
    mov x1, #12
    bl ctos_uart_write
    bl ctos_yield
    adr x0, msg_ok
    mov x1, #12
    bl ctos_uart_write
    mov x0, #0
    ret

    .align 3
msg_hi:
    .ascii "libctos: hi\n"
msg_ok:
    .ascii "libctos: ok\n"
