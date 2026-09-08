# Session memory — 2026-09-08 (sensors)

M2 exit is ARM semihosting SYS_EXIT / hlt #0xf000, not isa-debug-exit. QEMU needs `-semihosting`. force-fail feature proves non-zero. ubuntu-24.04-arm + ubuntu-24.04 matrix; do not pin Docker amd64. No docker binary on the cloud VM.
