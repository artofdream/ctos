# Session memory — 2026-09-11 (gaps to host apps / containers-no)

Docs + `research/` only on `cursor/docs-refresh-post-27-192e` (PR #29).

- **Host app** = Linux ELF, shell, Python, browser, OCI container you would `exec` on a host. None of those run in the guest.
- Gaps named: process/`exec`, POSIX libc, FS, net, TTY, isolation, preemption/SMP. Same Planned pieces as apps-today / filesystem / porting — not a new FR ID.
- **Containers: no.** No OCI/runc/cgroups/namespaces in `src/`. Host `docker-smoke.sh` is a build harness only. Do not write a k8s/Alpine-on-ctos guide.
- Ledger: host-apps row **Planned**; container-host row **Verified** absence and a **non-goal** (not Planned-as-later).
- Page: `docs/framework/host-apps.md`. Linked from README, overview, apps-today, building-or-porting, vision, moc.

Did not touch `src/` or smoke scripts. Do not treat this file as the honesty ledger.
