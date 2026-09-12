// Freestanding SVC stubs for the A1 ABI (ADR-021 / ADR-022).
// Immediate is the syscall number. Must not issue SVC #0, #1, or #2
// (those stay ADR-013 probes).

    .text
    .align 2

    .global ctos_exit
    .type ctos_exit, %function
ctos_exit:
    svc #16
1:
    b 1b

    .global ctos_uart_write
    .type ctos_uart_write, %function
ctos_uart_write:
    svc #17
    ret

    .global ctos_yield
    .type ctos_yield, %function
ctos_yield:
    svc #18
    ret
