#!/bin/sh
# Fail-closed public-identifier guard (ADR-093). Scans git-tracked text
# files for cloud-account-shaped identifiers that must not live in this
# public repo: AWS account IDs (12 digits next to an account/owner word,
# in an ARN, or in an ECR host), AWS access key IDs, EC2/VPC resource IDs,
# Spot request IDs and Route 53 hosted-zone IDs.
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
ALLOW='123456789012|111122223333|444455556666|AKIAIOSFODNN7EXAMPLE|i-1234567890abcdef0|sg-1234567890abcdef0'

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
    } >"$r/planted.md"
    git -C "$r" add planted.md
    scan "$r" "$tmp/hits"
    want='account-id-ctx account-id-arn account-id-ecr access-key-id ec2-resource-id spot-request-id route53-zone-id'
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
    if grep -q '^planted.md:9 ' "$tmp/hits" || grep -q '^planted.md:10 ' "$tmp/hits"; then
        echo "public-ids: self-test false positive (allowlist / bare number)" >&2
        exit 1
    fi
    echo "public-ids: self-test ok classes=7"
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
hits=$(wc -l <"$tmp/hits" | tr -d ' ')
if [ "$hits" -ne 0 ]; then
    echo "public-ids: FAIL hits=$hits (values not printed; replace with placeholders)" >&2
    sort -u "$tmp/hits" >&2
    exit 1
fi
echo "public-ids: ok files=$files hits=0"
