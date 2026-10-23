//! ADR-111 (N2): talk to the EL2 shim (`tools/el2-shim`) for a **virtual
//! SError** (HCR_EL2.VSE). Opt-in (`el2-vserror`). The shim only changes x0.
#![cfg(feature = "el2-vserror")]

const HVC_SHIM_MAGIC: u64 = 0x5E2E_1D00;

fn el2_implemented() -> bool {
    let pfr0: u64;
    unsafe { core::arch::asm!("mrs {}, id_aa64pfr0_el1", out(reg) pfr0, options(nomem, nostack)) };
    (pfr0 >> 8) & 0xf != 0
}

fn shim_call(cmd: u64) -> u64 {
    let mut x0 = cmd;
    unsafe { core::arch::asm!("hvc #0xc7", inout("x0") x0, options(nostack)) };
    x0
}

/// True only if EL2 is implemented and the shim answers its magic.
/// Without EL2 (`virtualization=off`) HVC is UNDEFINED, so it is not issued.
pub fn shim_present() -> bool {
    el2_implemented() && shim_call(0) == HVC_SHIM_MAGIC
}

/// Ask the shim to set HCR_EL2.VSE (a virtual SError becomes pending).
pub fn set_vse() -> bool {
    shim_call(1) == 0
}

/// Ask the shim to clear HCR_EL2.VSE (tidy after a masked or missed run).
pub fn clear_vse() {
    let _ = shim_call(2);
}
