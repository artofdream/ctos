# Session memory — 2026-09-27 — ADR-093 public-identifier scrub

- Branched from `main`, not the EL0 stack. ADR number 093 skips 087–092 (taken by open stack PRs).
- Placeholder convention: `<aws-account-id>`, `<route53-zone-id>`, `<instance-id-runN>`, `<spot-request-id-runN>`, `<security-group-id>`, `<iam-user>`, `<user>`. In Markdown, keep placeholders inside backticks, or mdBook eats them as HTML tags.
- Guard prints `file:line class` only. Never echo a matched value into public CI logs.
- A bare 12-digit number without context is intentionally not flagged (GitHub job IDs share the shape).
- Never write a real cloud identifier into commits, PR text, ADRs or comments. Describe by type and count.
