# ADR-113 — Own an arm64 KVM validation box

- Status: **Proposed.** No purchase is authorised by this document. The sponsor decides whether to buy, and what to buy. Do not self-merge ([ADR-002](ADR-002-pr-identity-split.md)).
- Date: 2026-11-21 (CET)
- Base: main `b5e7fcc` (after [#175](https://github.com/artofdream/ctos/pull/175)). ADR-112 is the last ADR on main, so this is ADR-113.
- Machine: amd64 workstation (docs only; throwaway clone). No cloud, no CloudAgent, no AWS call.
- Asked by: the user, 2026-11-21: "write an ADR on owning an arm64 box".
- Follows: [ADR-112](ADR-112-b3-graviton-real-hardware-serror-plan.md) (B3, host-chosen-syndrome **virtual SError** under KVM on Graviton metal), [ADR-085](ADR-085-b2p-graviton-kvm-serror.md), [ADR-095](ADR-095-g3-kvm-recheck.md), [ADR-099](ADR-099-kvm-recheck-ci-oidc-sketch.md).
- Not touched: `src/`, `user/`, `libctos/`, `Cargo.*`, workflows, the required checks, the ADR-091 rev 4 and ADR-106 sentences.
- Public-identifier rule ([ADR-093](ADR-093-public-identifier-scrub-guard.md)): no account, role, image, subnet, security-group or instance identifier appears here.

## Question

Should ctos own a real arm64 box, on the on-prem LAN, that can run the existing B3 / `b2-kvm` smoke so a **virtual SError** under KVM does not depend on AWS Spot capacity, environment approval, or a per-run cloud charge? If yes, which class of machine is the first candidate, and what must be true before Graviton is no longer the fallback?

## Why this is open

ADR-112's acceptance gate is already defined. A PASS needs `kvm-cap-serror-esr=1` and rows N / D / E on the existing smoke: row N parked (no SError), row D a **virtual SError** whose syndrome is `0xbf000000` exact, row E the same with the host-chosen nonce in the ISS (`0xbf000000 | N16` exact). If the capability is missing, the honest result is **INCONCLUSIVE**, not a pass.

That gate has been met once, on a supervised cloud run:

- [kvm-recheck run #5](https://github.com/artofdream/ctos/actions/runs/44392535730) on `main` `fec74d01` was **PASS** on `c7g.metal`, eu-north-1 Spot, attempt 1 of 9, on 2026-11-18 (job finished 22:52 CET). This is a **virtual SError** under KVM only. Not a physical SError, not hardware RAS, not the default image.
- Earlier supervised runs [43598194390](https://github.com/artofdream/ctos/actions/runs/43598194390), [44227097081](https://github.com/artofdream/ctos/actions/runs/44227097081) and [44339400638](https://github.com/artofdream/ctos/actions/runs/44339400638) were **SKIPPED-no-capacity** on `c7g.metal` Spot in every eu-north-1 AZ. Those are not a pass. The instance-type fallback `c7g.metal` → `m7g.metal` → `m6g.metal` (ADR-112 rev 2026-11-18, [#174](https://github.com/artofdream/ctos/pull/174)) was not needed on run #5.

The user wants to evaluate owning a box "to avoid dependency for validation": Spot capacity has been flaky, the `kvm-pilot` environment still needs a human approval, and even a cheap run is still a cloud spend and a teardown path. An owned box would join the on-prem fleet (LAN only). Credentials are not copied between machines.

A weekly shop watch (Norway + France) is scheduled for Mondays at about 09:00 CET. As of 2026-11-21 it has not run yet; the first run is 2026-11-23. Prices below are mid-November 2026 baselines already collected, not a live quote.

## Options

Facts that depend on third-party hardware or kernel support are cited. Where this project has not measured a machine, the text says so.

### A. Keep Graviton `.metal` Spot on demand (status quo)

The proven path. Run #5 showed the full N / D / E gate, including `kvm-cap-serror-esr=1`, for about cents of compute (the workflow compute cap was 0.33 USD; the exact Spot charge is still waiting on billing). The workflow already retries three AZs and, on `main`, three allowlisted metal types. Cost and teardown are capped; nothing is left running after a clean sweep.

The dependency remains: capacity can still come back as **SKIPPED-no-capacity**, and every dispatch waits on `kvm-pilot` approval. This option stays the **fallback** under every buy path below, until an owned box itself reports the cap and passes the same rows.

### B. Apple M1 or M2 MacBook, Asahi Linux

KVM on M1 under Asahi has been demonstrated since 2021: Linux as EL2, with VMs booting on KVM ([S1](#sources)). M1 and M2 remain the boring, already-shipping Asahi KVM hosts.

Public CPU-feature dumps from `sysregs -s` report **FEAT_RAS** yes (and FEAT_RASv1p1 / FEAT_RASv2 no) on M1, M3 and M4 ([S2](#sources)). The same document has no equivalent M2 `sysregs` table, so M2 RAS is not cited from that source. Linux advertises `KVM_CAP_ARM_INJECT_SERROR_ESR` only when `ARM64_HAS_RAS_EXTN` is set ([S3](#sources)). So an Asahi M1 is a *plausible* host for row E, but this project has **not** run the `b2-kvm` smoke on Asahi and has **not** read `kvm-cap-serror-esr` on one. Until that probe, do not treat a MacBook as a known-good ADR-112 box.

Asahi the OS is now generally available on M3 (installer no longer needs Expert mode; progress report November 2026, [S4](#sources)). That is not the same as "buy an M3 for KVM validation." KVM support for M3 (base/Pro/Max), base M4, A18 Pro and M5 Pro was applied to the KVM `next` tree on 2026-11-17 and is aimed at Linux 7.4 ([S5](#sources)). Prefer M1 or M2 for a first box.

Price guide, not a confirmed listing: a used M1 or M2 MacBook under about 6000 NOK / 500 EUR.

### C. Raspberry Pi 5 (8 GB or 16 GB)

KVM on a 64-bit Raspberry Pi OS is ordinary. The board is the cheapest way to stand up the existing smoke on the LAN and *read* the capability.

Whether row E can pass is **unproven**. `KVM_CAP_ARM_INJECT_SERROR_ESR` follows `ARM64_HAS_RAS_EXTN`. The Cortex-A76 TRM documents RAS as implementation-dependent: ID_AA64PFR0_EL1.RAS is 0x1 when the core has ECC, and 0x0 when it does not ([S6](#sources)). This project has not measured that bit, or `kvm-cap-serror-esr`, on a BCM2712 / Pi 5. If the cap is 0, the honest result for row E is **INCONCLUSIVE**. The box can still be useful for ordinary KVM work; it is not then the ADR-112 validation box.

Price guide: board under about 2400 NOK / 200 EUR, a small kit under about 3000 NOK / 250 EUR. Norway baselines seen in mid-November 2026: Proshop 2590 NOK (in stock), Kjell 2290–2390 NOK, MrSmart 3399 NOK, Multicom 5202 NOK (out of stock). The Multicom figure is an outlier; a laptop comparison should use the in-stock 2290–2590 NOK band, not 5202 NOK.

### D. Ampere Altra (or similar Neoverse workstation)

Architecturally the closest owned equivalent to the Graviton metal already used: a standard arm64 server CPU, KVM at EL2 without a special bootloader, and RAS is part of the Neoverse/Altra story, so `kvm-cap-serror-esr=1` is likely — still **unmeasured** here. Integrator listings for Altra workstations start around USD 6,000 (Avantek, checked 2026-11-21, [S7](#sources)); System76's Thelio Astra is in the same high-end desktop class ([S8](#sources)). Far above the used-laptop guide. Only if the sponsor wants a long-lived on-prem server.

### Rejected as the validation box

- **Snapdragon X laptops**, including the sponsor's 15.6-inch Galaxy Book4 Edge (base Snapdragon X) and consumer models such as Lenovo IdeaPad Slim 3x/5x, Yoga Slim 7x and ThinkPad T14s Gen 6. A stock Windows-on-ARM boot leaves Linux at EL1 under Qualcomm Gunyah, so there is no `/dev/kvm`. A community EL2 path (Secure Launch / `slbounce` and related trees) exists for some 14-inch Book4 Edge and T14s Gen 6 SKUs, but it is out of tree, depends on platform firmware artefacts, and is not a boring validation host ([S9](#sources)). Mainline device-tree work visible in late 2026 covers Galaxy Book4 Edge 14-inch and 16-inch "hamoa" SKUs, not the 15-inch model ([S10](#sources)).
- **Chromebooks** and **Android tablets**: not a usable, supported Linux-at-EL2 KVM host for this probe.

## Decision (proposed)

**Buy only after a cheap probe, or after a platform that is already a known-good KVM host is in budget — and keep Graviton as the fallback until the owned box itself passes the ADR-112 gate.**

Recommended candidate order:

1. **Do not retire Graviton.** Option A remains the only path this project has actually PASSed. Every owned box is extra until it reports `kvm-cap-serror-esr=1` and the existing `b2-kvm` smoke shows rows N / D / E.
2. **First purchase, if the sponsor wants to spend now: a Raspberry Pi 5 8 GB (option C), bought from an in-stock Norway listing in the ~2290–2590 NOK band, not the 5202 NOK outlier.** It is the cheapest way to put the smoke on the LAN and read the capability. If the cap is missing, stop and keep using Graviton; do not escalate the spend just to "have an arm64 box."
3. **Preferred longer-lived workstation, if a used M1 or M2 MacBook appears under about 6000 NOK / 500 EUR: option B (Asahi).** Stronger KVM history than the Pi; FEAT_RAS is documented on M1 (and M3/M4) but still unmeasured under this smoke. Prefer M1/M2 over M3+ for a first box. A laptop at that ceiling is also what the sponsor was weighing against an overpriced Pi listing; at the in-stock Pi prices the two are not the same spend.
4. **Ampere (option D) only if the sponsor deliberately wants a server-class machine.** Not the first buy.
5. **Do not buy a Snapdragon X laptop, Chromebook or Android tablet for this purpose.**

Any owned box joins the on-prem fleet (LAN only). No credentials are copied between machines.

The final buy, including "buy nothing and stay on Graviton," is the sponsor's.

## Acceptance

An owned box becomes the ADR-112 validation box only when:

- the host reports `kvm-cap-serror-esr=1`, and
- the existing `b2-kvm` smoke PASSes rows N, D and E (same markers as run #5: parked N, **virtual SError** exact syndromes on D and E).

Until then the box is a probe, and Graviton remains the fallback. A missing cap is **INCONCLUSIVE**, not a fail of ctos and not a pass.

## Consequences

- This PR is documentation only. No order is placed, no workflow changes, no cloud launch.
- The honesty-ledger row for an owned-box PASS stays **Planned** until a probe on the purchased (or already owned) machine lands.
- The Monday shop watch (Elkjøp, Power, NetOnNet, Komplett, finn.no in Norway; Leboncoin and Back Market in France) can feed a later revision with live prices. It must not silently turn this Proposed decision into a purchase.
- If a box is bought later, record the machine class, the cap, the row results and the cost in a follow-up ADR/ledger PR. Still a **virtual SError** under KVM unless a future probe honestly shows more.

## Sources

- [S1] Asahi Linux, "Progress Report: January / February 2021", KVM on M1: <https://asahilinux.org/2021/03/progress-report-january-february-2021/>. M1 feature-support page: <https://asahilinux.org/docs/platform/feature-support/m1/>.
- [S2] lelegard/arm-cpusysregs, "Arm features in the Apple M1 and M2 chips" (`sysregs -s` tables; FEAT_RAS yes on M1, M3, M4): <https://github.com/lelegard/arm-cpusysregs/blob/main/docs/apple-m1-features.md>.
- [S3] Linux KVM API, `KVM_CAP_ARM_INJECT_SERROR_ESR`; `kvm_vm_ioctl_check_extension` returns `cpus_have_final_cap(ARM64_HAS_RAS_EXTN)` (Linux `master`, `arch/arm64/kvm/arm.c`). Docs: <https://docs.kernel.org/virt/kvm/api.html>.
- [S4] Asahi Linux, "Progress Report: Linux 7.3" (M3 generally available): <https://asahilinux.org/2026/11/progress-report-7-3/>. Earlier "M2: Episode 1 (or, Asahi Linux on M3)": <https://asahilinux.org/2026/09/m2-episode-1/>. M3 feature-support: <https://asahilinux.org/docs/platform/feature-support/m3/>.
- [S5] "[PATCH v2 0/2] arm64: Add KVM support for newer Apple Silicon CPUs", applied to KVM `next` 2026-11-17, aimed at Linux 7.4: <https://lists.openwall.net/linux-kernel/2026/11/17/3723>. Commits `61615311ce39`, `3a7b70524c54` on `git.kernel.org/kvmarm/kvmarm`.
- [S6] Arm Cortex-A76 Technical Reference Manual, RAS / ID_AA64PFR0_EL1.RAS (0x1 when ECC present, 0x0 when not): <https://developer.arm.com/documentation/100798/0401/Reliability--Availability--and-Serviceability--RAS->.
- [S7] Avantek, "Ampere Altra Arm-based Workstation" listings (prices from about USD 6,000, checked 2026-11-21): <https://store.avantek.co.uk/amperer-altrar-arm-based-workstation.html>.
- [S8] Phoronix, "System76 Thelio Astra Reviewed: High-End ARM64 Developer Desktop": <https://www.phoronix.com/review/system76-thelio-astra>.
- [S9] Stock Snapdragon X firmware boots Linux at EL1 under Gunyah. Community EL2/KVM for some SKUs: n1c0la84/book4-edge-linux (14-inch Galaxy Book4 Edge notes, 2026-10-20, EL2 as a non-default boot chain); TravMurav/slbounce `x1e-el2` overlay; T14s Gen 6 write-ups such as rengare/t14s_x1e_linux.
- [S10] LKML, "[PATCH v7 3/3] arm64: dts: qcom: Add hamoa Samsung Galaxy Book4 Edge devicetrees" (14-inch and 16-inch SKUs): <https://lkml.iu.edu/2609.2/11777.html>.
