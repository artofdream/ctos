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
#
# ADR-112 (B3): a read-only RAS inventory (no module load, no /dev/mem, no
# write anywhere; KVM capability queries, the kernel boot log, ACPI table
# NAMES and the kernel config only), then three smoke rows in order:
#   N  no inject                       -> must park
#   D  inject, serror_has_esr = 0      -> must take, ESR_EL1 = 0xbf000000
#   E  inject, serror_has_esr = 1,     -> must take, ESR_EL1 = 0xbf00_0000|N16
#      ISS = 0x0100_0000 | N16 (N16 = low 16 bits of the nonce, 0x00c7 if 0)
# The guest only ever sees a virtual SError (HCR_EL2.VSE + VSESR_EL2).

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
# ---- ADR-112 read-only RAS inventory (values only, never table contents) ----
# KVM_CHECK_EXTENSION = _IO(0xAE, 0x03) on /dev/kvm: a query, changes nothing.
kvm_cap() {
  python3 -c "import fcntl,os,sys;print(fcntl.ioctl(os.open('/dev/kvm',os.O_RDWR),0xAE03,int(sys.argv[1])))" "$1" 2>/dev/null || echo err
}
if dmesg | grep -q 'RAS Extension Support'; then
  echo "CTOSB2: ras host-ext=yes"
else
  echo "CTOSB2: ras host-ext=no"
fi
# The cpufeature boot-log line itself (timestamp stripped; no host name in it).
dmesg | grep -m1 'RAS Extension Support' | sed -E 's/^\[[^]]*\] *//; s/^/CTOSB2: ras dmesg /' || true
# KVM_CAP_ARM_INJECT_SERROR_ESR = 158; KVM_CAP_ARM_INJECT_EXT_DABT = 178.
echo "CTOSB2: ras kvm-cap-serror-esr=$(kvm_cap 158)"
echo "CTOSB2: ras kvm-cap-ext-dabt=$(kvm_cap 178)"
acpi=""
for t in EINJ HEST AEST BERT SDEI; do
  if compgen -G "/sys/firmware/acpi/tables/${t}*" > /dev/null; then acpi="$acpi $t=yes"; else acpi="$acpi $t=no"; fi
done
echo "CTOSB2: ras acpi$acpi"
kcfg=n
if [ -r "/boot/config-$(uname -r)" ]; then
  kcfg=$(sed -n 's/^CONFIG_ACPI_APEI_EINJ=//p' "/boot/config-$(uname -r)" | head -1)
elif [ -r /proc/config.gz ]; then
  kcfg=$(zcat /proc/config.gz | sed -n 's/^CONFIG_ACPI_APEI_EINJ=//p' | head -1)
fi
echo "CTOSB2: ras kconfig APEI_EINJ=${kcfg:-n}"
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
  echo "CTOSB2: result smoke-rc=98 sha=none nonce=__NONCE__ rows=none"
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
# fail-closed smoke on ALL three rows counts as PASS; the workflow also needs
# each row's own PASS line, the exact row-E ESR, the commit and the nonce.
NONCE=__NONCE__
n16=$(( 16#${NONCE: -4} ))
[ "$n16" -eq 0 ] && n16=$(( 0xc7 ))
ISS_E=$(printf '0x%07x' $(( 0x1000000 | n16 )))
ESR_D=0xbf000000
ESR_E=$(printf '0x%08x' $(( 0xbf000000 | n16 )))
echo "CTOSB2: rows N,D,E n16=$(printf '0x%04x' "$n16") iss-e=$ISS_E expect-d=$ESR_D expect-e=$ESR_E"
# run_row NAME [VAR=VALUE ...]: up to 2 boots; prints `row NAME rc=..`.
run_row() {
  local name=$1 i r=99
  shift
  for i in 1 2; do
    echo "CTOSB2: ===== row $name attempt $i ====="
    env CTOS_QEMU="$QEMU_B2" CTOS_B2_TIMEOUT=90 CTOS_B2_STALL=45 "$@" \
      python3 scripts/b2-kvm-smoke.py "$ELF" 2>&1 | tr -d '\r' | sed 's/^/CTOSB2: /'
    r=${PIPESTATUS[0]}
    echo "CTOSB2: row $name attempt $i rc=$r"
    [ "$r" = 0 ] && break
  done
  ROW_RC=$r
}
rc=0
rows=""
for row in N D E; do
  case $row in
    N) run_row N CTOS_B2_NO_INJECT=1 ;;
    D) run_row D CTOS_B2_EXPECT_ESR=$ESR_D ;;
    E) run_row E CTOS_B2_SERROR_ESR="$ISS_E" CTOS_B2_EXPECT_ESR="$ESR_E" ;;
  esac
  if [ "$ROW_RC" = 0 ]; then rows="$rows,$row:ok"; else rows="$rows,$row:fail"; [ "$rc" = 0 ] && rc=$ROW_RC; fi
done
rows=${rows#,}
echo "CTOSB2: end $(date -u +%FT%TZ)"
sync
echo "CTOSB2: result smoke-rc=$rc sha=$(git rev-parse HEAD) nonce=$NONCE rows=$rows"
# Linger so the workflow can read the result while the instance still exists
# (it terminates the instance itself as soon as it has the result). The timer
# above is the backstop.
sleep 90
poweroff
