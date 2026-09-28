# Route 53 — `ctos.artof.link`

The CNAME is **already created** (sponsor, 2026-09-11). Public `dig` agrees. Do **not** `CREATE` again.

| Field | Value |
| --- | --- |
| Account | `<aws-account-id>` |
| Region | `us-east-1` |
| Hosted zone | `artof.link` / `<route53-zone-id>` |
| Record | `CNAME` `ctos.artof.link.` → `artofdream.github.io.` |

```bash
export AWS_REGION=us-east-1
test "$(aws sts get-caller-identity --query Account --output text)" = "${AWS_ACCOUNT_ID:?set AWS_ACCOUNT_ID locally}"
aws route53 list-resource-record-sets --hosted-zone-id "${ROUTE53_ZONE_ID:?set ROUTE53_ZONE_ID locally}" \
  --query "ResourceRecordSets[?Name=='ctos.artof.link.']"
```

Recovery batch only (if LIST shows the name missing): [`scripts/route53-ctos-cname.json`](../scripts/route53-ctos-cname.json).

DNS in place was the 2026-09-11 CNAME probe. HTTPS serving the book is a **separate** row: **Verified** after #30 (deploy [34653046584](https://github.com/artofdream/ctos/actions/runs/34653046584) + `curl -sSI https://ctos.artof.link` HTTP 200). Route 53 API LIST stays Unknown without AWS CLI. Full write-up: [`docs/website.md`](../docs/website.md).
