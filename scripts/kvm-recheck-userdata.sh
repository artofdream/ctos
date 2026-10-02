#!/bin/bash
# ADR-099 SKETCH (inert; nothing runs this until .github/workflows/kvm-recheck.yml
# is enabled by the sponsor path). Instance user-data for the KVM re-check:
# the ADR-095 (G3) variant of scripts/b2-metal-userdata.sh, plus a result line
# the workflow can parse fail-closed. Placeholders (replaced by the workflow
# with sed before launch): __SHA__ __NONCE__ __LIFETIME_MIN__.
#
# Everything is printed to the serial console as `CTOSB2: ...` lines. The
# workflow reads ONLY lines starting `CTOSB2: ` with ec2:GetConsoleOutput; it
# never prints the raw console (boot messages can carry the host name and
# addresses, and CI logs of a public repo are public).
#
# The instance has no instance profile, no key pair and no inbound rule: it
# holds no credentials and nothing can log in. Results leave only as console
# text (ADR-099 decision).

# Hard lifetime: power off (and, with shutdown behaviour `terminate`, terminate)
# even if a later step hangs. First line on purpose.
shutdown -h +__LIFETIME_MIN__ "ctos kvm-recheck hard timer" || true
exec > >(tee /dev/console /var/log/ctos-kvm-recheck.log) 2>&1
set -x
export HOME=/root
export PATH=/root/.cargo/bin:$PATH
echo "CTOSB2: start $(date -u +%FT%TZ)"
echo "CTOSB2: uname $(uname -srm)"   # no host name on the console
echo "CTOSB2: cpu $(lscpu | grep -E 'Model name|Vendor' | tr -s ' ' | tr '\n' ';')"
dmesg | grep -iE 'kvm|hyp|EL2' | sed 's/^/CTOSB2: dmesg /' | head -20
if [ -e /dev/kvm ]; then echo "CTOSB2: /dev/kvm=present"; else echo "CTOSB2: /dev/kvm=absent"; fi
echo "CTOSB2: kvm-api $(python3 -c "import fcntl,os;print(fcntl.ioctl(os.open('/dev/kvm',os.O_RDWR),0xAE00))" 2>&1)"
export DEBIAN_FRONTEND=noninteractive
apt-get update -qq
apt-get install -y -qq git curl build-essential ninja-build meson pkg-config python3-venv \
  flex bison libglib2.0-dev libpixman-1-dev libfdt-dev libslirp-dev >/dev/null
echo "CTOSB2: deps rc=$?"
curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain nightly \
  -c rust-src,llvm-tools-preview >/dev/null 2>&1
# shellcheck disable=SC1091
. /root/.cargo/env || true
# A setup failure still prints a result line (rc 98), so the workflow reports
# FAIL at once instead of waiting for the poll deadline.
die() {
  echo "CTOSB2: result smoke-rc=98 sha=none nonce=__NONCE__"
  sleep 60
  poweroff
  exit 1
}
cd /root || die
# Detached checkout of the exact commit the workflow resolved (reviewed, on main).
git clone https://github.com/artofdream/ctos.git ctos || die
cd ctos || die
git checkout --detach __SHA__ || die
echo "CTOSB2: head $(git rev-parse HEAD)"
CTOS_QEMU_JOBS=$(nproc) ./scripts/build-qemu-b2.sh > /var/log/ctos-qemu.log 2>&1
echo "CTOSB2: qemu-build rc=$?"
echo "CTOSB2: qemu-version $(tools/qemu-b2/bin/qemu-system-aarch64 --version | head -1)"
cargo build --features b2-serror --target-dir target-b2 > /var/log/ctos-cargo.log 2>&1
echo "CTOSB2: cargo rc=$?"
QEMU_B2=$PWD/tools/qemu-b2/bin/qemu-system-aarch64
ELF=target-b2/aarch64-ctos/debug/ctos
# 99 = the smoke never ran (build or setup failed). Only rc=0 from the
# fail-closed smoke counts as PASS; the workflow also needs the smoke's own
# PASS line and the commit and nonce below.
rc=99
for i in 1 2 3; do
  echo "CTOSB2: ===== smoke attempt $i ====="
  CTOS_QEMU=$QEMU_B2 CTOS_B2_TIMEOUT=90 CTOS_B2_STALL=15 \
    python3 scripts/b2-kvm-smoke.py "$ELF" 2>&1 | tr -d '\r' | sed 's/^/CTOSB2: /'
  rc=${PIPESTATUS[0]}
  echo "CTOSB2: smoke attempt $i rc=$rc"
  [ "$rc" = 0 ] && break
done
echo "CTOSB2: end $(date -u +%FT%TZ)"
sync
echo "CTOSB2: result smoke-rc=$rc sha=$(git rev-parse HEAD) nonce=__NONCE__"
# Linger so the workflow can read the result while the instance still exists
# (it terminates the instance itself as soon as it has the result). The timer
# above is the backstop.
sleep 90
poweroff
