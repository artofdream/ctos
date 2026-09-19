# Opt-in CI for ADR-083 pinned QEMU

Default `.github/workflows/smoke.yml` stays on distro QEMU (park path).

## Enable (sponsor / maintainer) — after `qemu-nmi-pin` is in smoke.yml

GitHub → **Settings** → **Secrets and variables** → **Actions** → **Variables** →
New repository variable:

- Name: `CTOS_BUILD_QEMU_NMI`
- Value: `1`

Unset or any other value skips the job. After enable, the next push/PR runs
build of the pinned QEMU and smoke with `CTOS_REQUIRE_TAKEN_SERROR=1`.

## Land the GHA job (blocked without `workflow` scope)

Editing `.github/workflows/smoke.yml` requires a credential with the OAuth
`workflow` scope. Box / Cursor GitHub OAuth currently has
`gist, read:org, repo` only. Exact push rejection:

```
refusing to allow an OAuth App to create or update workflow
`.github/workflows/smoke.yml` without `workflow` scope
```

**Unblock:** re-authorize the GitHub OAuth App (or use a PAT / EVO-X2 `gh`
login) with **`workflow`** + **`repo`**, then push the job below onto tip
`main` (~`8d05692` / ADR-083).

## Local / EVO-X2 (no GHA variable needed)

```sh
./scripts/build-qemu-nmi.sh
export CTOS_QEMU="$PWD/tools/qemu-nmi/bin/qemu-system-aarch64"
export CTOS_REQUIRE_TAKEN_SERROR=1
./scripts/qemu-smoke.sh
```

Dockerfile already supports `--build-arg CTOS_BUILD_QEMU_NMI=1`.

## Job to paste into smoke.yml

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
