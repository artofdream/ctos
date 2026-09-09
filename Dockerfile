# cts-ai host is Windows ARM64 → Docker Engine linux/arm64.
# Do not pin --platform=linux/amd64. ubuntu:24.04 is multi-arch.
FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive \
    CARGO_TERM_COLOR=always \
    RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH

RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates \
        curl \
        qemu-system-arm \
    && rm -rf /var/lib/apt/lists/*

# rust-toolchain.toml selects nightly; install it here so `docker run` is offline-ish.
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
        sh -s -- -y --default-toolchain nightly --profile minimal \
            --component rust-src --component llvm-tools-preview

WORKDIR /src
COPY . .

# Strip CR so a Windows-context `docker build` cannot ship `#!/bin/sh\r`.
# `.gitattributes` (`*.sh text eol=lf`) is the checkout ratchet; this is
# the image ratchet. `bash file` also ignores a broken shebang.
RUN find scripts -name '*.sh' -exec sed -i 's/\r$//' {} + \
    && chmod +x scripts/*.sh

CMD ["bash", "./scripts/qemu-smoke.sh"]
