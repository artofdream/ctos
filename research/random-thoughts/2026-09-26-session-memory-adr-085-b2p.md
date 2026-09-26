# Session memory — ADR-085 B2-P (2026-09-26)

- Sanctioned AWS path = MCP `user-Aws-mcp` `aws___run_script`. Python sandbox: `import os` and `import base64` are blocked. `GetConsoleOutput` comes back already decoded. Long multi-call scripts can time out **after** side effects: always re-describe after a timeout, and prefer one mutating call per script.
- Launch guard pattern: describe `tag:project=ctos` in pending/running/stopping/stopped/shutting-down → refuse to launch. Metal `shutting-down` → `terminated` takes 5–10 min.
- Graviton metal boots to user-data in ≈20 s after console appears. Patched QEMU build ≈4 min on 64 cores, ctos debug build ≈2 min, whole run ≈7–12 min.
- cloud-init: `HOME` is unset. Export it before rustup.
- KVM on Graviton3: GICv3 only (`disabling GICv2 emulation`). The b2 profile skips GIC/timer.
- **TCG hides real-hardware bugs.** Reserved `TCR_EL1.TG1=0b00` is sanitised to 4K by QEMU. Real Neoverse-V1 differs. Next real-HW candidates if the full image ever runs under KVM: GIC (v3 only), timer, virtio-mmio, cache maintenance for DMA.
- A stall dump via QMP `human-monitor-command "info registers"` works under KVM. A PC at VBAR+0x200 with DAIF masked means a recursive current-EL sync abort.
- Hypothesis discipline: the pre-MMU exclusives idea was plausible but refuted by the control in the same run. The change was reverted rather than kept "just in case".
