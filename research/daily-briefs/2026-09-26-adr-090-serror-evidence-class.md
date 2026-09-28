# 2026-09-26 — ADR-090 (M4): taken-SError evidence class (sponsor D1)

- **Decision relayed:** DSO, 2026-09-26 ~14:35 CEST. D1 = (i)+(ii): B1 pinned-QEMU CI job plus the B2-P one-off KVM run, written verbatim into the accept ADR.
- **Done:** ADR-090 carries the verbatim evidence paragraph (ADR-091 must quote it unchanged). `qemu-nmi-pin` is now unconditional: the `vars.CTOS_BUILD_QEMU_NMI` gate is gone and the job is renamed “QEMU NMI pin smoke (taken SError)”. The workflow edit was pushed from EVO-X2 (the box OAuth lacks `workflow`).
- **Why unconditional:** D5 makes the smoke a required check before the flip, and a job skipped by `if:` satisfies a required check on GitHub.
- **Row status:** ADR-055 §B row 1 reworded and Met on the D1 class. Stock TCG still parks (default smoke requires `el0: serror-park`).
- **Caveats:** B1 = locally patched emulator. B2-P = historical at `159b178` (before M1/M2), reduced profile. KVM re-verify ≈ $0.2, not done (sponsor option).
- **Still not:** umbrella accept (B3). M5 drafts ADR-091, and the sponsor must read it. No flip (M6). No “EL0 isolated”.
- **M6 due item (D5):** branch protection on `main` requires `QEMU aarch64 smoke (ubuntu-24.04-arm)`, `QEMU aarch64 smoke (ubuntu-24.04)`, `QEMU NMI pin smoke (taken SError)`. Sponsor/DSO sets it; not changed by the agent.
