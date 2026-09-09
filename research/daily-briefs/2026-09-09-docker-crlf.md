# Daily brief — 2026-09-09 (Docker ratchets)

## Where we stopped

PR #4. cts-ai `docker build` linux/arm64 Verified. `docker run` Failed three times: CRLF shebang, missing host `cc`, then missing `efi-virtio.rom`. LF + `.gitattributes` + `build-essential` + `ipxe-qemu`/`qemu-efi-aarch64` landed. Full image smoke after all three ratchets is Unknown until re-probe.

## Do next

1. Rebuild and re-probe `docker run --rm ctos-smoke` on cts-ai after pull.
2. Human or MRC review. Author does not merge (ADR-002).
