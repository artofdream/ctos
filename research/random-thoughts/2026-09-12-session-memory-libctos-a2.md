# Session memory — 2026-09-12 A2 libctos

- Chose `no_std` crate + asm stubs (`libctos/src/sys.S`) over a C static lib. Hello is `user/hello-libctos` linked at `0x80002000` (`paging::EL0_PAGE`).
- Host `build.rs` cargo-builds the hello crate with target name `aarch64-ctos-user` so kernel `-Tlinker.ld` rustflags do not apply. Extracts PT_LOAD at/after EL0_PAGE (drops rust-lld’s 0x80000000 ELF-header load). Image is 108 bytes.
- CRT `_start` in `crt0.S`: `bl main` / `b ctos_exit`. Kernel ERET sets SP.
- Probe payload must not issue SVC #0/#1/#2. Build.rs and a `#[test_case]` scan encodings.
- Rust `main` needs a writable SP. A1’s EL0-exec page is AP=00 (EL0 fetch, no EL0 data). WXN blocks making that page W+X. A2 maps `EL0_PAGE+0x1000` EL0-RW NX (`DESC_AP_EL0`).
- Not A3: no guest ELF loader. Not A4: standing EL0 still a probe trip.
