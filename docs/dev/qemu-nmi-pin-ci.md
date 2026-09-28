# CI for ADR-083 pinned QEMU (unconditional since ADR-090)

The default `qemu-smoke` jobs in `.github/workflows/smoke.yml` stay on distro QEMU (park path).

Job `qemu-nmi-pin` (“QEMU NMI pin smoke (taken SError)”) builds the pinned QEMU and runs the smoke with `CTOS_REQUIRE_TAKEN_SERROR=1` on **every push and pull request** ([ADR-090](../03-adr/ADR-090-serror-evidence-class.md), sponsor D1/D5). Until ADR-090 it was gated by repo variable `CTOS_BUILD_QEMU_NMI == '1'`. That gate was removed because GitHub treats a job skipped by `if:` as passing a required check. The variable is now unused; the Dockerfile build-arg of the same name is unrelated and stays opt-in.

Workflow edits need a credential with the OAuth `workflow` scope (EVO-X2 `gh` login). The box OAuth app has `gist, read:org, repo` only.

## Local / EVO-X2 (no GHA variable needed)

```sh
./scripts/build-qemu-nmi.sh
export CTOS_QEMU="$PWD/tools/qemu-nmi/bin/qemu-system-aarch64"
export CTOS_REQUIRE_TAKEN_SERROR=1
./scripts/qemu-smoke.sh
```

Dockerfile already supports `--build-arg CTOS_BUILD_QEMU_NMI=1`.

## Job as landed (reference; see smoke.yml for the source of truth)

```yaml
  qemu-nmi-pin:
    name: QEMU NMI pin smoke (taken SError)
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
