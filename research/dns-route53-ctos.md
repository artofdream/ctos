# Route 53 — `ctos.artof.link`

The CNAME is **already created** (sponsor, 2026-09-11). Public `dig` agrees. Do **not** `CREATE` again.

| Field | Value |
| --- | --- |
| Account | `737290977112` |
| Region | `us-east-1` |
| Hosted zone | `artof.link` / `Z1178AFMV41RWP` |
| Record | `CNAME` `ctos.artof.link.` → `artofdream.github.io.` |

```bash
export AWS_REGION=us-east-1
test "$(aws sts get-caller-identity --query Account --output text)" = "737290977112"
aws route53 list-resource-record-sets --hosted-zone-id Z1178AFMV41RWP \
  --query "ResourceRecordSets[?Name=='ctos.artof.link.']"
```

Recovery batch only (if LIST shows the name missing): [`scripts/route53-ctos-cname.json`](../scripts/route53-ctos-cname.json).

DNS in place ≠ docs live. Enable GitHub Pages + repo custom domain after merge. Do not claim `https://ctos.artof.link` serves the book until that HTTPS probe. Full write-up: [`docs/website.md`](../docs/website.md).
