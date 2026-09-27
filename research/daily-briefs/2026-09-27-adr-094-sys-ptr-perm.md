# 2026-09-27 — ADR-094: G4 closed (syscall pointers need EL0 permission)

- **Decision path:** the sponsor skipped the ADR-091 revision-2 G4 question. The DSO applied the standing order: close G4 before bringing the sentence back (relay ~12:06 CEST).
- **Fix:** `user_range_ok_max(ptr, len, max, Read|Write)` checks the user leaf's AP bits on **every** page:
  - read → `AP[1]`
  - write → `AP[1]` and `!AP[2]`
  - execute-only, kernel-only and unmapped pages fail with the syscall's error value
  - `SYSPTR_DENIED` counts refusals made by this check, and probes need exactly 1 per call
- **Loader:** `Perm::Exec` maps text+rodata **EL0 RO+X** (`map_el0_text`: AP=11, UXN clear, PXN set), so apps can print their own `.rodata`. A2 `run_hello` does the same. The app ELF is unchanged (same sha256; A9 cross-update ok). Kernel probe payloads keep execute-only code and use a new EL0-RW NX data page (the `crt-stack-pan` slot).
- **Probes (default smoke):**
  - `control rw-data n=6`
  - `denied kernel-stub` / `kernel-window` (`SYS_UART_WRITE`)
  - `denied xo-text` / `write-ro` (`SYS_NET_MAC` copy_to_user, sentinel unchanged)
  - `denied straddle` (second page kernel-only)
  - a seven-syscall sweep (17/19/20/27/22 read the stub; 21/24 write own xo text)
  - `sweep syscalls=7 denied=7`, `ok`
- **Leak build (`sysptr-leak-probe`, with reach/write leak features):** `leaked kernel-stub n=8` (8 stub bytes on the UART), `leaked kernel-window n=8`, `leaked xo-text n=6 intact=false checks=0`, `skip write-ro`, no `ok` → smoke caught.
- **G1 allowlist:** unchanged (5 slots); the live walk output is byte-identical.
- **ADR-091 revision 3:** the carve-out is dropped and a syscall-pointer clause (S9) added. G3 is the only remaining limit (optional). Still Draft; B3 not Met.
- **New limits stated:** one-core check-then-copy (no `AT S1E0*`/`LDTR`/`STTR`, no SMP TOCTOU story). App text is EL0-readable now.
