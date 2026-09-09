# Session memory — 2026-09-09 (Docker CRLF)

cts-ai: `docker build -t ctos-smoke .` on Docker Desktop linux/arm64 succeeded. `docker run --rm ctos-smoke` failed: `exec ./scripts/qemu-smoke.sh: no such file or directory` — `#!/bin/sh\r` after a Windows checkout. Ratchet: `.gitattributes` `*.sh`/`Dockerfile` `eol=lf`; Dockerfile `sed` strip CR + `CMD ["bash","./scripts/qemu-smoke.sh"]`. Full image smoke after the fix unprobed here.
