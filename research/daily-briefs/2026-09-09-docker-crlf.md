# Daily brief — 2026-09-09 (Docker CRLF ratchet)

## Where we stopped

PR #4. cts-ai built the linux/arm64 image; `docker run` exec failed on a CRLF shebang. LF + `.gitattributes` + image `sed` landed; full smoke after the fix is Unknown until re-probe.

## Do next

1. Re-probe `docker run --rm ctos-smoke` (or `bash ./scripts/qemu-smoke.sh`) on cts-ai after pull.
2. Human or MRC review. Author does not merge (ADR-002).
