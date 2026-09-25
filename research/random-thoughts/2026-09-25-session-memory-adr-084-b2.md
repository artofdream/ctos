# Session memory — ADR-084 B2 free-runner spike

- The existing smoke already runs on `ubuntu-24.04-arm`, but under TCG. "Native arm64 runner" ≠ "KVM". The dmesg line `kvm [1]: HYP mode not available` is the one-line answer: Azure boots the arm64 guest kernel at EL1, so no KVM.
- `inject-nmi` is a *machine* feature (`TYPE_NMI` object), not an accelerator feature. Even with KVM, stock QEMU virt would still say `machine does not provide NMIs`. The honest KVM route is `KVM_SET_VCPU_EVENTS` `serror_pending` (virtual SError via `HCR_EL2.VSE`), which stock QEMU never exposes → needs a tiny custom VMM.
- macOS runners: M1 VMs, `kern.hv_support` sysctl missing, HVF `HV_UNSUPPORTED`. Even with HVF, Hypervisor.framework has only IRQ/FIQ pending types — no SError.
- windows-11-arm: no VT exposed; WHPX feature disabled and would need a reboot.
- x86 `/dev/kvm` is present on `ubuntu-24.04` (Android emulator story) — irrelevant for AArch64 guests.
- Probe script `qemu-nmi-probe.py` races QEMU 6.2 (no cortex-a76) → ConnectionRefused on 22.04-arm. Harmless; could harden later.
- CopyFromBox to EVO-X2 can only drop into `C:\Users\cts`; move with PowerShell afterwards.
- Temp probe branch `probe/adr-084-b2-kvm` pushed from EVO-X2 (workflow scope). Runs persist after branch deletion.
