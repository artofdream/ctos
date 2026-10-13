#!/bin/sh
# Fail-closed public-identifier guard (ADR-093). Scans git-tracked text
# files for cloud-account-shaped identifiers that must not live in this
# public repo: AWS account IDs (12 digits next to an account/owner word,
# in an ARN, or in an ECR host), AWS access key IDs, EC2/VPC resource IDs,
# Spot request IDs and Route 53 hosted-zone IDs.
# ADR-100 adds host / user classes: agent VM host names, EC2 private host
# names, host names inside uname lines, GitHub runner host names, user home
# paths, WSL -u user names, UUIDs (machine IDs), and retired machine names.
# Machine names are matched by SHA-256 of the lower-cased token, so this
# file does not spell them out (a hash of a short name can be brute-forced;
# this keeps the names out of plain-text search, it is not secrecy).
#
# This file holds generic patterns only, never a real value. On a hit it
# prints file:line and the class, NOT the matched text (CI logs are public).
# Put real values in local config or CI secrets and write a placeholder
# here, e.g. `<aws-account-id>` or "${AWS_ACCOUNT_ID}".
#
#   sh scripts/check-public-ids.sh              # scan the repo
#   sh scripts/check-public-ids.sh --self-test  # planted tokens must be caught
set -eu

# Public AWS documentation example values only. Never add a real value here.
ALLOW='123456789012|111122223333|444455556666|AKIAIOSFODNN7EXAMPLE|i-1234567890abcdef0|sg-1234567890abcdef0|/home/runner|-u root'

# SHA-256 of retired machine names (lower case). ADR-100. Never add a
# plain-text name here.
NAME_HASHES='e604d7f1aef8fc520698b611efb1972873cc7e52dd5d01b9a79884c667b8f020 6d5be622942356f7679e404951ec776b8473ea063011a6b301ccf559f26517c3 695b675832ecadd6a62c3bb85a0e0a8fdf3a7cf1faa9b52901775cfffb581e43'
EXTRA_HASHES=''

B='(^|[^0-9A-Za-z_-])'
BH='(^|[^0-9A-Za-z_])'
E='([^0-9A-Za-z_]|$)'
D12='[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]'

# class|git-grep-flags|ERE
rules() {
    cat <<RULES
account-id-arn|-E|arn:aws[a-z-]*:[a-z0-9-]*:[a-z0-9-]*:${D12}:
account-id-ctx|-E -i|(account|acct|owner[ _-]?id)[^0-9]{0,40}${D12}([^0-9]|\$)
account-id-ecr|-E|${D12}\\.dkr\\.ecr\\.
access-key-id|-E|(AKIA|ASIA|AIDA|AROA|AGPA|ANPA|ANVA|AIPA)[A-Z0-9]{16}${E}
ec2-resource-id|-E|${BH}(i|sg|subnet|vol|vpc|eni|snap|ami|igw|rtb|nat|eipalloc|lt)-[0-9a-f]{8}([0-9a-f]{9})?${E}
spot-request-id|-E|${B}sir-[0-9a-z]{8}${E}
route53-zone-id|-E|(hosted-zone-id|HostedZoneId|hostedzone/|[Zz]one|ZONE)([ _-]?(ID|Id|id))?[^0-9A-Za-z]{0,20}Z[0-9A-Z]{9,31}([^0-9A-Za-z]|\$)
route53-zone-id|-E|${BH}Z[0-9][0-9A-Z]{9,31}${E}
vm-hostname|-E -i|[a-z0-9-]*-vm-[0-9]{6,}
ec2-private-hostname|-E|${BH}ip-[0-9]{1,3}-[0-9]{1,3}-[0-9]{1,3}-[0-9]{1,3}${E}
uname-hostname|-E|(Linux|Darwin) [A-Za-z][A-Za-z0-9.-]*[0-9][A-Za-z0-9.-]* [0-9]+\\.[0-9]+
gha-runner-hostname|-E|runnervm[a-z0-9]{3,}
user-home-path|-E|([A-Za-z]:[\/]+Users[\/]+|/Users/|/home/)[A-Za-z0-9._-]+
wsl-user|-E -i|wsl(\\.exe)? [^|]*-u [A-Za-z0-9._-]+
uuid|-E -i|${BH}[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}${E}
RULES
}

# scan DIR -> prints "path:line class" per hit; returns hit count via file
scan() {
    dir=$1
    out=$2
    : >"$out"
    rules | while IFS='|' read -r class flags re; do
        # shellcheck disable=SC2086
        git -C "$dir" grep -I -n -o $flags -e "$re" -- . ':!scripts/check-public-ids.sh' 2>/dev/null |
            while IFS= read -r hit; do
                m=${hit#*:}
                m=${m#*:}
                if printf '%s\n' "$m" | grep -E -q "$ALLOW"; then
                    continue
                fi
                loc=${hit%"$m"}
                printf '%s %s\n' "${loc%:}" "$class" >>"$out"
            done || true
    done
}

# Retired machine names, by hash: every short hyphenated word is hashed and
# compared; on a match only "path:line machine-name" is written.
scan_names() {
    dir=$1
    out=$2
    git -C "$dir" grep -I -h -o -w -i -E '[A-Za-z]{2,6}-[A-Za-z0-9]{1,4}' -- . ':!scripts/check-public-ids.sh' 2>/dev/null |
        tr '[:upper:]' '[:lower:]' | sort -u >"$tmp/tok" || true
    while IFS= read -r t; do
        h=$(printf '%s' "$t" | $SHA | cut -c1-64)
        case " $NAME_HASHES $EXTRA_HASHES " in
        *" $h "*)
            git -C "$dir" grep -I -n -i -w -F -e "$t" -- . ':!scripts/check-public-ids.sh' 2>/dev/null |
                cut -d: -f1,2 | sed 's/$/ machine-name/' >>"$out" || true
            ;;
        esac
    done <"$tmp/tok"
}

if command -v sha256sum >/dev/null 2>&1; then
    SHA=sha256sum
elif command -v shasum >/dev/null 2>&1; then
    SHA='shasum -a 256'
else
    echo "public-ids: no sha256sum/shasum on PATH (fail closed)" >&2
    exit 1
fi

if ! command -v git >/dev/null 2>&1; then
    echo "public-ids: git not on PATH (fail closed)" >&2
    exit 1
fi

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT INT TERM

if [ "${1:-}" = "--self-test" ]; then
    # Synthetic tokens built at runtime (not real, not in any file here).
    r=$tmp/repo
    mkdir -p "$r"
    git -C "$r" init -q
    n12=$(printf '%s%s' 424242 424242)
    hex17=$(printf '%s%s' 0123456789abcdef 0)
    {
        echo "account $n12"
        echo "arn:aws:iam::$n12:role/x"
        echo "$n12.dkr.ecr.us-east-1.amazonaws.com"
        echo "key AKIA$(printf 'Q%.0s' 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16)"
        echo "inst i-$hex17"
        echo "spot sir-abcd1234"
        echo "zone ZQ$(printf '%s' 12345678901)"
        echo "run-i-$hex17 and \`Z9$(printf '%s' 12345678901)\`"
        echo "doc example account 123456789012"
        echo "plain run id $n12 without context"
        echo "host=agent$(printf '%s' -vm-)1234567"
        echo "CTOSB2: uname Linux ip-10-1-2-3 6.8.0-1 #1 SMP aarch64"
        echo "host runnervm$(printf '%s' abc12)"
        printf 'path C:\\Users\\alice%s\\x and /home/bob%s/y\n' x x
        echo "wsl -d Alpine -u carol$(printf x) -- bash"
        echo "id 0123abcd-0123-4567-89ab-0123456789ab"
        echo "built on ZQ-X9 and zq-x9"
        echo "ok /home/runner/work wsl -d Alpine -u root -- apk; uname Linux 7.0.0-1-aws aarch64; C:\\Users\\<user>\\"
    } >"$r/planted.md"
    git -C "$r" add planted.md
    EXTRA_HASHES=$(printf '%s' zq-x9 | $SHA | cut -c1-64)
    scan "$r" "$tmp/hits"
    scan_names "$r" "$tmp/hits"
    want='account-id-ctx account-id-arn account-id-ecr access-key-id ec2-resource-id spot-request-id route53-zone-id vm-hostname ec2-private-hostname uname-hostname gha-runner-hostname user-home-path wsl-user uuid machine-name'
    if [ "$(grep -c '^planted.md:8 ' "$tmp/hits")" -lt 2 ]; then
        echo "public-ids: self-test missed hyphen-prefixed instance / bare zone" >&2
        exit 1
    fi
    for c in $want; do
        if ! grep -q " $c\$" "$tmp/hits"; then
            echo "public-ids: self-test missed $c" >&2
            exit 1
        fi
    done
    if grep -q '^planted.md:9 ' "$tmp/hits" || grep -q '^planted.md:10 ' "$tmp/hits" || grep -q '^planted.md:18 ' "$tmp/hits"; then
        echo "public-ids: self-test false positive (allowlist / bare number)" >&2
        exit 1
    fi
    echo "public-ids: self-test ok classes=15"
    exit 0
fi

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
if ! git -C "$ROOT" rev-parse --git-dir >/dev/null 2>&1; then
    echo "public-ids: $ROOT is not a git checkout (fail closed)" >&2
    exit 1
fi
files=$(git -C "$ROOT" ls-files | wc -l | tr -d ' ')
if [ "$files" -eq 0 ]; then
    echo "public-ids: no tracked files (fail closed)" >&2
    exit 1
fi
scan "$ROOT" "$tmp/hits"
scan_names "$ROOT" "$tmp/hits"
hits=$(wc -l <"$tmp/hits" | tr -d ' ')
if [ "$hits" -ne 0 ]; then
    echo "public-ids: FAIL hits=$hits (values not printed; replace with placeholders)" >&2
    sort -u "$tmp/hits" >&2
    exit 1
fi
echo "public-ids: ok files=$files hits=0"
