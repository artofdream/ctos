# Route 53 playbook — `ctos.artof.link`

Procedure for the **next** session that has AWS credentials in account **737290977112**. This file is not a claim that the record was written from here.

- Hosted zone: `artof.link` (region **us-east-1** assumed for the CLI session; Route 53 API is global)
- Record: `CNAME` `ctos.artof.link.` → `artofdream.github.io.` (trailing dot)
- Batch file: [`scripts/route53-ctos-cname.json`](../scripts/route53-ctos-cname.json)
- Full steps + GitHub Pages custom-domain / Enforce HTTPS: [`docs/website.md`](../docs/website.md)

```bash
export AWS_REGION=us-east-1
test "$(aws sts get-caller-identity --query Account --output text)" = "737290977112"
ZONE_ID=$(aws route53 list-hosted-zones-by-name --dns-name artof.link. \
  --query "HostedZones[?Name=='artof.link.'].Id" --output text)
ZONE_ID="${ZONE_ID##*/}"
aws route53 list-resource-record-sets --hosted-zone-id "$ZONE_ID" \
  --query "ResourceRecordSets[?Name=='ctos.artof.link.']"
# Only if missing:
aws route53 change-resource-record-sets --hosted-zone-id "$ZONE_ID" \
  --change-batch file://scripts/route53-ctos-cname.json
```

Do **not** mark custom-domain reachability Verified until Pages lists `ctos.artof.link` and `curl -sSI https://ctos.artof.link` returns 200 with a matching cert.
