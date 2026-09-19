# Opt-in CI for ADR-083 pinned QEMU

Default `.github/workflows/smoke.yml` stays on distro QEMU (park path).

To enable taken-SError CI, add a job (requires a token with `workflow` scope
to edit the workflow file) or run locally / on EVO-X2:

```sh
./scripts/build-qemu-nmi.sh
export CTOS_QEMU="$PWD/tools/qemu-nmi/bin/qemu-system-aarch64"
export CTOS_REQUIRE_TAKEN_SERROR=1
./scripts/qemu-smoke.sh
```

Suggested workflow job (paste when a `workflow`-scoped push is available):

```yaml
  qemu-nmi-pin:
    name: QEMU NMI pin smoke (opt-in)
    if: ${{ vars.CTOS_BUILD_QEMU_NMI == '1' }}
    runs-on: ubuntu-24.04
    timeout-minutes: 90
    steps:
      - uses: actions/checkout@v4
        with: { fetch-depth: 0 }
      - uses: dtolnay/rust-toolchain@nightly
        with: { components: rust-src, llvm-tools-preview }
      - run: |
          sudo apt-get update
          sudo apt-get install -y ninja-build meson pkg-config python3-venv flex bison \
            libglib2.0-dev libpixman-1-dev libfdt-dev libslirp-dev
      - run: ./scripts/build-qemu-nmi.sh
      - env:
          CTOS_QEMU: ${{ github.workspace }}/tools/qemu-nmi/bin/qemu-system-aarch64
          CTOS_REQUIRE_TAKEN_SERROR: "1"
        run: ./scripts/qemu-smoke.sh
```

Dockerfile already supports `--build-arg CTOS_BUILD_QEMU_NMI=1`.
