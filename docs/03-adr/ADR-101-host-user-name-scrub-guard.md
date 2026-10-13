# ADR-101 — Forward-only scrub of host and user names + guard classes

- Status: Accepted on merge (security / compliance guardrail, sponsor decision 2026-10-13 20:49 CEST, relayed by DSO). Not a kernel change.
- Date: 2026-10-13
- Follows: [ADR-093](ADR-093-public-identifier-scrub-guard.md) (cloud identifiers + `scripts/check-public-ids.sh`). ADR-099 is taken by the open draft PR #152; this ADR takes 100.
- **No history rewrite** (sponsor: forward-only). The replaced names remain in earlier commits and merged PR diffs.
- `_start` (`0x4008_0000`) untouched. No kernel logic change: `src/` and `linker.ld` change in comments only.

## Context

ADR-093 scrubbed cloud identifiers and the local OS user name in host paths. It did not classify **machine names** (the nicknames of the two developer workstations, used in docs as if they were roles), **host names** that tools printed into committed logs (an agent VM host name, an EC2 private host name, GitHub-hosted runner host names inside `uname -a` lines) or a **machine ID** (a UUID, in full once and as an 8-hex prefix four times). This repository is public. Two scripts would also print a host name into future logs: `scripts/b2-metal-userdata.sh` (`uname -a`) and `scripts/qemu-nmi-probe.py` (`socket.gethostname()`).

## Decision

1. **Replace with role names or placeholders** (values are not listed here):
   - the amd64 developer workstation → “amd64 workstation”; the arm64 developer workstation → “arm64 workstation” (with “the” where the grammar needs it);
   - agent VM host name → `<agent-vm-hostname>`; EC2 private host name → `<ec2-private-hostname>`; GitHub runner host names → `<gha-runner-hostname>`; machine ID → `<machine-id>`.
   “agent-box” stays: it is a role name, not a host name.
2. **Stop new leaks at the source:** `b2-metal-userdata.sh` prints `uname -srm` (as ADR-095 already did by hand); `qemu-nmi-probe.py` prints `host-os=<sysname> <machine>` instead of the host name.
3. **Guard classes (fail closed, CI-gated through `scripts/qemu-smoke.sh` as before).** `scripts/check-public-ids.sh` gains eight classes: `vm-hostname`, `ec2-private-hostname`, `uname-hostname` (a host-name-shaped token between `Linux`/`Darwin` and a version), `gha-runner-hostname`, `user-home-path` (`C:\Users\<name>`, `/Users/<name>`, `/home/<name>`; `/home/runner` and `<placeholder>` allowed), `wsl-user` (`wsl … -u <name>`; `root` and `<placeholder>` allowed), `uuid`, and `machine-name`. Machine names are matched by **SHA-256 of the lower-cased token**, so the guard does not spell them out. That keeps them out of plain-text search; a hash of a short name can be brute-forced, so it is not secrecy. On a hit the guard still prints only `path:line class`. `--self-test` now plants a synthetic token for all 15 classes (including a runtime-hashed fake machine name in two cases) plus a line of allowed forms that must not be flagged; marker `public-ids: self-test ok classes=15`.

## Counts (by class, no values)

Guard of this branch run on unmodified `main` `af5a9d1` (distinct `path:line class` pairs) and a plain grep for occurrences:

| Class | Before (lines) | Before (occurrences) | After |
| --- | --- | --- | --- |
| machine-name (two workstation names) | 129 | 148 (88 + 60) | 0 |
| vm-hostname | 7 | 7 | 0 |
| ec2-private-hostname | 1 | 1 | 0 |
| uname-hostname (overlaps the two rows above and below) | 5 | 5 | 0 |
| gha-runner-hostname | 3 | 4 | 0 |
| uuid (machine ID, runner host) | 2 | 2 | 0 |
| machine-ID 8-hex prefix (not a guard class; grep) | 4 | 4 | 0 |
| hostname-printing code | 2 scripts | 2 | 0 |

New guard on `main`: `FAIL hits=151`. On this branch: `public-ids: ok … hits=0` (bash and dash).

## Limits (non-claims)

- Shapes and a short hashed name list, not DLP. A new machine name needs its hash added. Short role-like words are not flagged.
- Git history, PR descriptions, issue comments and CI logs are not scanned or changed.
- Git author metadata and the GitHub owner name in repo URLs stay (public and intended).
- Not a “secure” or “compliant” claim.
