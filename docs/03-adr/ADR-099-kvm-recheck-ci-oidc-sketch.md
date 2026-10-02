# ADR-099 — Milestone item 2: the KVM re-check in CI via GitHub OIDC (design sketch, inert)

- Status: **Proposed — design only.** Nothing in this ADR is implemented. **No AWS call was made and no AWS resource was created or changed** to write it. The workflow it adds, [`.github/workflows/kvm-recheck.yml`](../../.github/workflows/kvm-recheck.yml), is **inert** (`workflow_dispatch` only, not scheduled, not a required check, fails closed while its role variable is unset). The sponsor and DSO decide; do not self-merge ([ADR-002](ADR-002-pr-identity-split.md)).
- Date: 2026-10-02 (CEST).
- Follows: [ADR-085](ADR-085-b2p-graviton-kvm-serror.md) (first KVM run), [ADR-090](ADR-090-serror-evidence-class.md) (evidence class), [ADR-091](ADR-091-el0-isolated-accept-draft.md) (Accepted; its sentence is **not touched**), [ADR-093](ADR-093-public-identifier-scrub-guard.md) (no identifiers in public text), [ADR-095](ADR-095-g3-kvm-recheck.md) (G3: the manual run this automates).
- Not in this ADR: no `src/`, `user/`, `libctos/` or `Cargo.*` change; `_start` (`0x4008_0000`) untouched; `smoke.yml`, `pages.yml` and the in-tree `scripts/b2-metal-userdata.sh` untouched; the three required checks are unchanged; CloudAgent not used.
- **No threat-model bump.** `security.md` on `main` is v1.72 (ADR-098, PR #151, merged by the sponsor on 2026-10-02 13:01 CEST); v1.71 and v1.73 are reserved by the open PRs #149 and #150. A bump here would collide with those numbers. The risk table below is written so DSO can attach it to the sponsor record; folding it into `security.md` is a follow-up after #149 and #150 merge.
- Public-identifier rule ([ADR-093](ADR-093-public-identifier-scrub-guard.md)): every account, role, instance, image, subnet, security-group and zone identifier is a `<PLACEHOLDER>` here and in the workflow. Real values belong in repository **variables** set by the sponsor-approved path, never in repo text, commits, PR bodies or logs.

## Context

[ADR-095](ADR-095-g3-kvm-recheck.md) proved by hand (one `c7g.metal` Graviton3 Spot instance, about **$0.04**, reduced `b2-serror` profile, markers `el0: serror` + `b2: taken`) that the taken lower-EL SError still happens under KVM. That run is a point in time. Milestone item 2 asks whether the same check can run **from CI** so the claim can be re-checked on a later commit without a person at an AWS console.

GitHub-hosted runners cannot run KVM for this profile on Graviton metal, so CI would have to start a short-lived AWS instance. That needs AWS credentials in CI. The credential rule (rule 4) forbids stored keys. The sponsor approved **one exception, tracked as PX0**: a **GitHub OIDC role with no stored keys** (DSO relay, 2026-09-28 21:23 CEST). The exception is conditional. Before the role is created **or used**:

- **(a)** a CloudTrail trail is on;
- **(b)** the ctos branch protection on `main` is enforced for admins;
- **(c)** how test results leave the instance is decided **before** asking for any extra permission such as `iam:PassRole`.

Condition (c) is answered in this ADR (section 2). The conditions above are taken from the DSO relay text. The decision record in the DSO vault (`01-second-brain/decisions/2026-09-28-ctos-ci-oidc-exception.md`) **was not read** while writing this ADR: the file exists on the DSO machines, but the executor's file reader refused a path outside its allowed root, and that refusal was not worked around. Nothing here quotes or paraphrases the record, and the text of credential rule 4 is not reproduced. **DSO must reconcile this ADR with the record before it goes to the sponsor.**

## Observations made while writing (GitHub read-only, 2026-10-02 about 13:00 CEST; no AWS)

| Item | Observed | Meaning |
| --- | --- | --- |
| Branch protection on `main`, **enforce for admins** | **On** | Condition (b) is **verified** today. Re-verify at role-creation time. |
| Required checks on `main` | The three smoke checks (`ubuntu-24.04-arm`, `ubuntu-24.04`, NMI pin) | The new workflow is **not** one of them and must not become one. |
| Required approving reviews on `main` | **None** (only status checks are required); `strict` is off | A PR that passes the three checks can be merged by anyone with merge rights. In practice the sponsor merges (ADR-002), but the control is by convention, not by GitHub. Listed as a residual risk below. |
| Force-push and branch deletion on `main` | Not allowed | Good. |
| Actions environments | Only `copilot` and `github-pages`. **No KVM environment exists.** | A workflow that names an environment that does not exist makes GitHub **create it with no protection**. The environment must be created (reviewers, main-only branch policy) **before** the role exists. |
| Repository variables | Only one build variable; **no `KVM_*` variable** | The workflow is inert: its guard fails on the unset role variable. |
| OIDC `sub` claim customisation | The repository uses the default template **with an immutable subject**; the subject prefix carries the numeric owner and repository IDs | The `sub` GitHub emits is **probably not** the plain `repo:<owner>/<repo>:…` form that most AWS examples show. A trust policy written from an example would **never match** (fails closed, but also never works). The exact string must come from the documentation and the customisation setting at role-creation time, and be confirmed in the supervised pilot from the CloudTrail `AssumeRoleWithWebIdentity` event. **Not verified here.** |
| Repository visibility | **Public** | Actions logs are public. See "Identifier leakage" in the risk table. |

## Decision (proposed)

Add an **opt-in, manual, inert** workflow that, once the sponsor path enables it, assumes a least-privilege OIDC role, launches **one** Spot `c7g.metal` instance, reads the result **from the serial console only**, and **terminates** the instance in an `always()` step, with three independent teardown layers and a hard cost cap. Only a result produced by a probe on the instance is ever reported as PASS.

### 1. Trust policy and permission policy (placeholders only)

#### 1.1 Trust policy shape

`StringEquals` only. **No `StringLike`, no wildcard.** The audience is fixed, the subject names this repository and exactly one of: the `main` branch **or** one named environment. The environment form is recommended, because the environment can also require a human reviewer and a main-only branch policy.

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Sid": "GitHubOidcOnlyThisRepoOnlyThisEnvironment",
      "Effect": "Allow",
      "Principal": {
        "Federated": "arn:aws:iam::<ACCOUNT_ID>:oidc-provider/token.actions.githubusercontent.com"
      },
      "Action": "sts:AssumeRoleWithWebIdentity",
      "Condition": {
        "StringEquals": {
          "token.actions.githubusercontent.com:aud": "sts.amazonaws.com",
          "token.actions.githubusercontent.com:sub": "<SUB_FOR_THIS_REPO_AND_ENVIRONMENT>"
        }
      }
    }
  ]
}
```

`<SUB_FOR_THIS_REPO_AND_ENVIRONMENT>` is one exact string. Two shapes, by the claim template in force:

| Template in force | Environment form (recommended) | Branch form (alternative) |
| --- | --- | --- |
| Classic | `repo:<OWNER>/<REPO>:environment:<ENV_NAME>` | `repo:<OWNER>/<REPO>:ref:refs/heads/main` |
| **Immutable subject (what this repository reports) [unverified format]** | `repo:<OWNER>@<OWNER_ID>/<REPO>@<REPO_ID>:environment:<ENV_NAME>` | `repo:<OWNER>@<OWNER_ID>/<REPO>@<REPO_ID>:ref:refs/heads/main` |

`<OWNER_ID>` and `<REPO_ID>` are GitHub's numeric IDs, read from the repository settings by the person creating the role, and **not** written to the repo. Do not "fix" a mismatch by widening to `StringLike` or `*`. A mismatch is a **fail-closed** AccessDenied; fix the string.

Other properties: role maximum session **3600 s**; the OIDC identity provider for `token.actions.githubusercontent.com` must already exist in the account (precondition P4); the audience is the default that `aws-actions/configure-aws-credentials` requests. Optional hardening: customise the subject or add a `job_workflow_ref` condition so only `kvm-recheck.yml` on `main` matches **[verify the claim and the customisation in the AWS and GitHub docs before relying on it]**.

Why the environment form: a pull request from a fork, or from any branch, produces a different `sub` (a `pull_request` subject, or another ref) and does not match. A `workflow_dispatch` from `main` bound to the environment matches only after the environment's protection rules pass.

#### 1.2 Permission policy (least privilege)

Placeholders: `<REGION>` (one region, the one used in ADR-095), `<ACCOUNT_ID>`, `<AMI_ID>` (pinned Ubuntu 24.04 arm64 image), `<SUBNET_ID_1>` … `<SUBNET_ID_n>` (one subnet per AZ), `<SG_ID>` (a security group **pre-created** by DSO: tagged, **no ingress**).

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Sid": "RunInstancesNewInstanceOnly",
      "Effect": "Allow",
      "Action": "ec2:RunInstances",
      "Resource": "arn:aws:ec2:<REGION>:<ACCOUNT_ID>:instance/*",
      "Condition": {
        "StringEquals": {
          "ec2:InstanceType": "c7g.metal",
          "ec2:InstanceMarketType": "spot",
          "ec2:MetadataHttpTokens": "required",
          "aws:RequestTag/project": "ctos",
          "aws:RequestTag/component": "kvm-recheck"
        },
        "ForAllValues:StringEquals": { "aws:TagKeys": ["project", "component", "run"] }
      }
    },
    {
      "Sid": "RunInstancesNewChildResourcesMustBeTagged",
      "Effect": "Allow",
      "Action": "ec2:RunInstances",
      "Resource": [
        "arn:aws:ec2:<REGION>:<ACCOUNT_ID>:volume/*",
        "arn:aws:ec2:<REGION>:<ACCOUNT_ID>:network-interface/*",
        "arn:aws:ec2:<REGION>:<ACCOUNT_ID>:spot-instances-request/*"
      ],
      "Condition": {
        "StringEquals": {
          "aws:RequestTag/project": "ctos",
          "aws:RequestTag/component": "kvm-recheck"
        }
      }
    },
    {
      "Sid": "RunInstancesPinnedInputsOnly",
      "Effect": "Allow",
      "Action": "ec2:RunInstances",
      "Resource": [
        "arn:aws:ec2:<REGION>::image/<AMI_ID>",
        "arn:aws:ec2:<REGION>:<ACCOUNT_ID>:subnet/<SUBNET_ID_1>",
        "arn:aws:ec2:<REGION>:<ACCOUNT_ID>:subnet/<SUBNET_ID_n>"
      ]
    },
    {
      "Sid": "RunInstancesTaggedNoIngressSecurityGroupOnly",
      "Effect": "Allow",
      "Action": "ec2:RunInstances",
      "Resource": "arn:aws:ec2:<REGION>:<ACCOUNT_ID>:security-group/<SG_ID>",
      "Condition": { "StringEquals": { "ec2:ResourceTag/project": "ctos" } }
    },
    {
      "Sid": "TagOnCreateOnly",
      "Effect": "Allow",
      "Action": "ec2:CreateTags",
      "Resource": [
        "arn:aws:ec2:<REGION>:<ACCOUNT_ID>:instance/*",
        "arn:aws:ec2:<REGION>:<ACCOUNT_ID>:volume/*",
        "arn:aws:ec2:<REGION>:<ACCOUNT_ID>:network-interface/*",
        "arn:aws:ec2:<REGION>:<ACCOUNT_ID>:spot-instances-request/*"
      ],
      "Condition": { "StringEquals": { "ec2:CreateAction": "RunInstances" } }
    },
    {
      "Sid": "TerminateAndReadConsoleOfOurTaggedInstancesOnly",
      "Effect": "Allow",
      "Action": ["ec2:TerminateInstances", "ec2:GetConsoleOutput"],
      "Resource": "arn:aws:ec2:<REGION>:<ACCOUNT_ID>:instance/*",
      "Condition": {
        "StringEquals": {
          "ec2:ResourceTag/project": "ctos",
          "ec2:ResourceTag/component": "kvm-recheck"
        }
      }
    },
    {
      "Sid": "DescribeInstancesForPollAndSweep",
      "Effect": "Allow",
      "Action": "ec2:DescribeInstances",
      "Resource": "*",
      "Condition": { "StringEquals": { "aws:RequestedRegion": "<REGION>" } }
    },
    {
      "Sid": "DenyOtherRegions",
      "Effect": "Deny",
      "Action": "ec2:*",
      "Resource": "*",
      "Condition": { "StringNotEquals": { "aws:RequestedRegion": "<REGION>" } }
    },
    {
      "Sid": "DenyIamS3SsmExplicitly",
      "Effect": "Deny",
      "Action": ["iam:*", "s3:*", "ssm:*"],
      "Resource": "*"
    }
  ]
}
```

(The last Deny is belt and braces: nothing here allows those services, but an explicit Deny keeps a later, careless policy attachment from widening the role. It deliberately does not name `kms:*`: if account-default EBS encryption uses a customer-managed key, the launch may need KMS permissions, and that is a decision for the sponsor, not a hidden Deny.)

**Exact actions to launch, tag-scope and terminate one Spot instance:**

| Action | Needed for | Can the `project=ctos` tag restrict it? |
| --- | --- | --- |
| `ec2:RunInstances` | launch the one instance (with Spot market options, in one call) | **Partly.** On `instance`, `volume`, `network-interface`, `spot-instances-request` through `aws:RequestTag/project` (and `component`), i.e. the request **must** tag what it creates. On the security group through `ec2:ResourceTag/project`. **Not** on the image or the subnets (they are not tagged by us; pinned by ARN instead). |
| `ec2:CreateTags` | tag at creation | **Scoped, not by the tag value**: `ec2:CreateAction = RunInstances` means tags can only be written during a launch, never onto an existing resource. |
| `ec2:TerminateInstances` | teardown | **Yes**, `ec2:ResourceTag/project` + `component` on the instance. |
| `ec2:GetConsoleOutput` | the result channel | **Yes**, same condition. |
| `ec2:DescribeInstances` | poll state, Spot interruption reason, sweeper | **No.** `Describe*` has no resource-level scope; it is limited by region only. It leaks nothing the account's EC2 console would not already show the role holder. |
| `iam:PassRole` | an instance profile | **Not requested.** There is no instance profile. |
| `iam:CreateServiceLinkedRole` | first-ever Spot use | **Not requested.** The Spot service-linked role must pre-exist (P8, unverified). |
| `ec2:CreateSecurityGroup`, `ec2:Authorize*`, key pairs, SSM, S3, KMS | — | **Not requested.** The group is pre-created. |
| `ec2:DescribeImages`, `DescribeSubnets`, `DescribeSpotPriceHistory` | — | **Not requested.** Image and subnets are pinned; `MaxPrice` is the cap. |

Service-reference check (public AWS documentation, fetched 2026-10-02, not an AWS API call): `TerminateInstances` and `GetConsoleOutput` act on the `instance` resource type, which supports `ec2:ResourceTag/*`; `RunInstances` touches the instance, volume, network-interface, spot-instances-request, security-group, subnet and image types; `Describe*` have no resource types. **IAM condition behaviour is not exercised until a supervised pilot** (P13); expect one or two adjustments, each of which goes back to the sponsor as a change to this ADR.

Unverified, flagged for the pilot: if the account's default EBS encryption uses a customer-managed key, `RunInstances` may need extra KMS permissions; if so, that is a **new** permission request and goes to the sponsor under condition (c) logic, it is not added silently.

### 2. Results-out decision (condition (c))

The question: how does "the smoke passed" leave a box that has no credentials, no key pair and no inbound port?

| Option | What it needs | What it avoids / costs | Verdict |
| --- | --- | --- | --- |
| **A. Serial console, `ec2:GetConsoleOutput`** | One tag-scoped `Get` on the instance (already in the policy). No instance profile. | **Avoids `iam:PassRole`, any instance profile, any bucket, any agent, any credential on the instance.** The instance cannot send anything except text to its own console. Cost: nothing. Limits: last 64 KiB, may lag, text only, **boot noise on the console can carry the host name or addresses** (so the workflow prints only `^CTOSB2: ` lines). | **Recommended.** It is exactly what ADR-085 and ADR-095 used. |
| B. SSM (Run Command / session output) | An instance profile with SSM core permissions, hence **`iam:PassRole`**, plus `ssm:SendCommand`, `ssm:GetCommandInvocation`, an agent on the box and an output path. | Adds a new principal on the instance, a new service, a bigger role. | Rejected. It buys a bigger channel than a one-line verdict needs. |
| C. S3 bucket (instance PUTs the result) | Either an instance profile (**`iam:PassRole`**) or a **pre-signed PUT URL** minted by the role (no PassRole, but `s3:PutObject` and a bucket). | A **new resource class** (bucket, lifecycle, public-block settings, cost), a URL that is a bearer secret on the instance and in user-data, a read path. | Not now. Kept as a **fallback** if console output proves unreliable in the pilot; that would be a new sponsor decision under (c). |
| D. Instance pushes to GitHub (status, artefact, comment, self-hosted runner) | A GitHub token (or runner registration token) **on the instance**. | **Violates the credential rule's intent**: a long-lived or minted secret on a machine that downloads and builds third-party code. | Rejected. |

**Recommendation: A, console output.** It needs the least (two tag-scoped EC2 read/terminate actions the role needs anyway), avoids the most (PassRole, instance profile, S3, SSM, any credential on the instance), and was already proven by hand twice.

How the workflow stays honest with a text-only channel:

- **Result contract.** The user-data prints `CTOSB2: result smoke-rc=<rc> sha=<commit> nonce=<random>` last (`rc` starts at 99 = "never ran"; a setup failure prints 98). The nonce is generated by the workflow and substituted into the user-data, so a stale console or another run's text cannot be mistaken for this run.
- **PASS needs all of:** the result line with `rc=0`, the exact commit and nonce; an anchored `CTOSB2: b2-kvm-smoke: PASS taken lower-EL SError observed under accel=kvm` line (anchored, so a `set -x` echo of the same words does not count); `/dev/kvm=present`; `uname` reporting `aarch64`; the KVM API-version line. Anything else is FAIL. No result line before the deadline is INCONCLUSIVE.
- **Live vs after termination (unverified).** It is not verified here whether the console is readable live or only after the instance stops. The design handles both: the instance lingers about 90 s after printing the result; the workflow polls live, and if the instance is already terminated it makes a few more reads before giving up as INCONCLUSIVE.
- **Local test of the parser** (no AWS): the parse block was run against the real ADR-095 console excerpt plus synthetic result lines. PASS only with the right commit, nonce, smoke line and markers; wrong nonce, wrong commit, rc≠0, missing smoke line, KVM absent, an unanchored echo and a setup failure all give FAIL; no result gives INCONCLUSIVE. This tests the parser, not AWS.
- **Public logs.** The workflow prints only the filtered marker lines and AWS error **codes**; it never prints raw console text or raw AWS stderr. Instance IDs are masked with `::add-mask::` as soon as they exist.

### 3. Fail-safe teardown and cost cap

Four layers, so no single failure leaves a metal instance running:

1. **Pre-flight refusal.** If any `project=ctos` instance is active, the run stops with `REFUSED-active-instance` before spending anything. Concurrency group `kvm-recheck` (`cancel-in-progress: false`) serialises runs.
2. **On the instance:** `shutdown -h +<LIFETIME_MIN>` is the first line of the user-data (default **45 min**; the guard refuses anything over 60), shutdown behaviour is `terminate`, and the script powers off after a 90 s linger. This works even if the runner dies.
3. **Always-run terminate step.** `if: always()`, with **fresh credentials** (a second `configure-aws-credentials`, guarded by the first one's success, because the first session may have expired or been cancelled). It finds the run's instances **by tag** (`run=<run id>`), so a launch that died after `RunInstances` but before returning an ID is still cleaned up, terminates them, and waits for `terminated`.
4. **Tag-based sweeper check (same step).** Counts every non-terminated `project=ctos` instance. Any left over gives result **LEAK** and a red job. This is a check inside the run, **not** a scheduled sweeper: a schedule would use the role with no human present and needs its own sponsor decision.

Job `timeout-minutes` is 60; the poll limit is 40 min.

**Hard cost cap per run.** Spot is billed at the market price, never above `MaxPrice`. The guard computes `cap = MAX_PRICE_USD_HR × (LIFETIME_MIN + 5) / 60` and **refuses to run** if it exceeds `RUN_COST_CAP_USD`, or if the price is above the ceiling the sketch allows. Defaults: `MaxPrice` $0.40/h (the observed Spot price in ADR-095 was $0.2476/h, the manual run used a $0.60 ceiling), lifetime 45 min, cap $0.50; the computed worst case is **about $0.33** per run [estimate]. EBS (30 GB gp3, minutes) adds under $0.01 [estimate]. A price above `MaxPrice` interrupts the instance, which ends the spend and gives INCONCLUSIVE, not PASS.

**Typical cost** [estimate]: **$0.03–0.07 per run** (ADR-095: about 8 minutes of metal at about $0.25/h, about $0.04; build time varies with Spot price and apt mirrors).

| Scenario | Runs / month | Monthly cost [estimate] |
| --- | --- | --- |
| Manual, as needed | 4–8 | about $0.12–0.56 |
| Weekly schedule (**not enabled, not proposed here**) | about 4–5 | about $0.12–0.35 |
| After every merge to `main` (**not proposed**) | about 20 | about $0.6–1.4 |
| Worst case: every run hits the full cap | n | about $0.34 × n |

CloudTrail cost for one management-events trail is small [unverified]. An **AWS Budget alarm** at a low monthly figure is recommended as a last backstop (P9, optional, unverified).

**Residual:** if the GitHub runner dies hard **and** the user-data failed to start, the instance runs until `MaxPrice` interrupts it or someone notices (up to about $0.40 per hour). The in-instance timer, the pre-flight refusal on the next run, and the budget alarm are the mitigations.

### 4. Spot capacity-failure story

`KVM_SUBNET_IDS` is a space-separated list, **one subnet per AZ**, tried in order. On each failed `RunInstances` the workflow reads only the AWS error **code**:

| Code class | Examples | Action |
| --- | --- | --- |
| Capacity | `InsufficientInstanceCapacity`, `SpotMaxPriceTooLow`, capacity-not-available | try the next AZ |
| Quota / limit | `MaxSpotInstanceCountExceeded`, `VcpuLimitExceeded` | stop: `SKIPPED-no-capacity` (reason: quota) |
| Anything else (for example AccessDenied) | | `FAIL` immediately; a permissions problem is not "no capacity" |
| All AZs exhausted | | `SKIPPED-no-capacity` |

**A skip is never green.** The job exits non-zero with a step summary that says "not a pass". There is **no on-demand fallback**: a metal on-demand instance would break the cost cap. A mid-run Spot interruption (`StateReason` of Spot termination) is INCONCLUSIVE and is **not** retried automatically. The ledger records the **last PASS commit and date**; a skipped or inconclusive run never updates it.

Result words: **PASS** (the only exit 0), **FAIL**, **INCONCLUSIVE**, **SKIPPED-no-capacity**, **REFUSED-active-instance**, **LEAK**, **NOT-RUN** (guard refused, no AWS call).

### 5. Honesty: what a PASS proves

A PASS proves **only** this: at the printed commit on `main`, on one Graviton3 `.metal` host with `/dev/kvm`, under QEMU v10.0.0 + patches 0001/0002 with `accel=kvm`, the **reduced `b2-serror` profile** printed `el0: serror` and `b2: taken` (the lower-EL SError was taken).

It does **not** prove: the default image boots or behaves the same under KVM; the identity inventory, reach walk or syscall-pointer probes; anything about real hardware beyond this one profile; or that "EL0 isolated" is wider than the accepted ADR-091 sentence (that sentence is not changed here). It is also not a security claim. Status words in the summary and the ledger come from the probe's result line only, never from the workflow's own opinion. The code under test controls its own console output; that is inherent to the method, so every run records the commit it tested.

### 6. Precondition checklist (DSO and sponsor)

Nothing below is done by this PR. "State" is the state **in this PR's evidence**; all items must be re-checked at the time they matter.

| # | Precondition | Owner | State (2026-10-02) |
| --- | --- | --- | --- |
| P1 | **(a)** A CloudTrail trail is on (management events, all regions or at least `<REGION>`) | DSO | **NOT VERIFIED** (no AWS call is allowed here) |
| P2 | **(b)** Branch protection on `main` enforced for admins | DSO / sponsor | **Verified** via the GitHub API. Note: no required approving review; `strict` is off. |
| P3 | **(c)** Results-out decided before any extra permission | Sponsor accepts this ADR | **Decided in this ADR (console output)**, **pending sponsor accept**. No PassRole requested. |
| P4 | The GitHub OIDC identity provider exists in the account | DSO | **NOT VERIFIED** |
| P5 | A GitHub environment (placeholder name `kvm-recheck`) with **required reviewers** and a **main-only deployment branch policy**, created **before** the role | Sponsor / DSO | **Absent** today |
| P6 | The role created via a **sponsor-approved path** (not by an agent) with exactly the trust and permission policies above, adjusted for the immutable subject | Sponsor / DSO | Not done |
| P7 | Pinned inputs set as repository **variables** (never repo text): image, no-ingress tagged security group, one subnet per AZ, region, role ARN | DSO | Not done (all unset) |
| P8 | Spot service-linked role and Spot vCPU quota for one `c7g.metal` | DSO | **NOT VERIFIED** |
| P9 | AWS Budget alarm (optional backstop) | DSO | **NOT VERIFIED** / optional |
| P10 | Credential rule 4 and the PX0 exception record referenced in the PR that enables the workflow | DSO | **Record not read** for this ADR; rule text not reproduced here |
| P11 | `KVM_ROLE_ARN` is set **last** | DSO | Unset, so the workflow is inert (verified) |
| P12 | Third-party actions pinned by full commit SHA | Author, before enabling | Not done (tags in the sketch) |
| P13 | A **supervised pilot** run, then DSO reviews CloudTrail (who assumed what, which calls, which denies) | DSO + sponsor | Not done |
| P14 | The exact immutable-subject `sub` string confirmed | DSO | **NOT VERIFIED** |
| P15 | Public-log hygiene confirmed on the pilot (no identifier in the log or summary) | DSO | Not done |

### 7. Risk and impact evaluation (for the sponsor record)

Impact is "what an attacker or a mistake could cost". Residual is after the listed mitigations, assuming P1–P15 are met.

| # | Threat | Likelihood × impact (before) | Mitigations | Residual risk |
| --- | --- | --- | --- | --- |
| 1 | **Another repo, branch, fork or pull request assumes the role** | Low × High | Exact `aud` and `sub` (no wildcard); environment with required reviewers and main-only branch policy; fork PRs get a different `sub` and no id-token for secrets | Low. Depends on the exact `sub` being right (P14) and the environment existing first (P5). |
| 2 | **A PR edits the workflow to abuse the role** | Medium × High | Dispatch runs only the default-branch file; guard refuses any ref but `main`; environment gate; `enforce_admins` is on | **Low–Medium.** `main` requires no approving review (observed), so the control is the sponsor merging by convention. DSO may ask the sponsor to require one review on `main` or a code-owner rule for `.github/workflows/`. |
| 3 | **Role is over-permissioned (miner, wrong instance type, other region)** | Medium × High | Conditions on instance type, Spot market, IMDSv2, tags and region; pinned image/subnets/SG; 1 h session; no IAM, no PassRole; Spot quota; budget alarm | Low. IAM conditions are untested until the pilot (P13). |
| 4 | **Tag spoofing** (a resource tagged `project=ctos` to dodge the sweeper, or the terminate scope abused) | Low × Low | Tags only at creation (`CreateAction`); terminate only matches tags the role itself applied; the sweeper counts the tag broadly | Low. A person with other credentials could still tag things; out of scope for this role. |
| 5 | **Privilege escalation through the instance** | Low × High | No instance profile, no `iam:PassRole`, explicit Deny on `iam:*`; IMDSv2 required, hop limit 1; no credentials on the box | Low. |
| 6 | **Secret in user-data** | Low × Medium | User-data holds only a commit, a nonce and a timer; no keys, tokens or URLs with secrets | Very low. |
| 7 | **Identifier leakage in public logs** (instance ID, account ID, host name, IP) | Medium × Low–Medium | Account-ID masking; instance IDs masked at creation; only `^CTOSB2: ` lines and error **codes** are printed; no raw console or stderr; identifier guard on repo text | Low. Confirm on the pilot (P15). |
| 8 | **Leaked running instance (cost, exposure)** | Medium × Low | Pre-flight refusal; in-instance `shutdown -h +45` and terminate-on-shutdown; always-run terminate with fresh credentials, by tag; wait for terminated; sweeper gives LEAK; budget alarm | Low. Worst case about $0.40/h until noticed if the runner and the user-data both fail. |
| 9 | **Console text is forged or replayed** | Low × Medium | Per-run nonce and commit in the result line; anchored PASS line; KVM and arch markers; terminate-by-tag so one run's console is read once | Low. The code under test can print its own PASS (inherent; the commit is recorded and the tested commit must be on `main`). |
| 10 | **Silent green on a skip or timeout** | Medium × High (trust) | Only PASS exits 0; SKIPPED, INCONCLUSIVE, REFUSED, LEAK, NOT-RUN are red; guard fails (not skips) on an unset variable | Low. |
| 11 | **Spot capacity or interruption** | High × Low | Multi-AZ retry; honest SKIPPED / INCONCLUSIVE; no on-demand fallback | Accepted: the check is sometimes unavailable. |
| 12 | **Third-party action supply chain** (`configure-aws-credentials`, `checkout`) | Low × High | Pin by full commit SHA before enabling (P12); `permissions: {}` at top level; the job only gets `id-token: write` and `contents: read` | Low–Medium until pinned. |
| 13 | **Supply chain on the instance** (apt, the rustup install script, git clone, QEMU source) | Medium × Low | QEMU is built from the pinned v10.0.0 source plus the repo's patches; the repo is cloned at a commit already on `main`; the instance holds **no credentials**, has no ingress, IMDSv2 hop limit 1, and lives under an hour | Low. Open egress remains (needed for apt/rustup/git). |
| 14 | **Repo rename or transfer** (subject reuse by a new owner) | Low × High | The immutable subject (owner and repo IDs) is what the trust policy pins | Low once P14 is confirmed. |
| 15 | **Cost runaway** | Low × Low | Cost-cap guard; MaxPrice; lifetime; concurrency 1; budget alarm | Low; about $0.33 worst case per run [estimate]. |
| 16 | **CloudTrail off, so no audit trail** | Unknown × High | Condition (a) is a hard precondition before the role is **created or used** | Open until P1 is verified. |
| 17 | **Unprotected environment auto-created** by the workflow's first reference | Medium × High | P5: create the environment with protections first; order matters | Low if the order is followed. |
| 18 | **Unknown account facts** (Spot service-linked role, quota, EBS default key) | Medium × Low | P8; pilot; any new permission goes to the sponsor, not added silently | Low. |
| 19 | **Credential rule 4 is only relaxed for this role** | Low × Medium | One role, one repository, one environment, no stored key, 1 h tokens; the exception record is referenced in the enabling PR (P10) | Low. Any second role or a schedule needs its own approval. |

### 8. The inert workflow

[`.github/workflows/kvm-recheck.yml`](../../.github/workflows/kvm-recheck.yml) and [`scripts/kvm-recheck-userdata.sh`](../../scripts/kvm-recheck-userdata.sh) are the sketch.

- **Triggers:** `workflow_dispatch` only (one optional input, the commit to test, which must be on `main`). No `schedule`. It is not, and must not become, a required check.
- **Fails closed:** the first step makes no AWS call and **fails** (red, summary "NOT-RUN") unless the repository, the ref (`refs/heads/main`), the role variable (and image, group, subnets, region variables), the role ARN shape and the cost arithmetic are all in order. While `KVM_ROLE_ARN` is unset, it **cannot** do anything else.
- **Permissions:** `permissions: {}` at the top; the job gets `id-token: write` and `contents: read` only.
- **Pinned inputs come from variables**, not repo text.
- **Pre-run checks done locally (no AWS):** YAML parse, `actionlint` with `shellcheck`, `bash -n` and `shellcheck` on the user-data, the guard run with simulated variables (unset, wrong ref, wrong repo, bad ARN, over-cap, hostile input), and the console parser run on the real ADR-095 excerpt (see section 2).
- **Sketch status:** the workflow has **never run**; the `ubuntu-24.04` runner's AWS CLI behaviour, the `aws-actions` inputs, and every AWS call are untested.

User-data is derived from the ADR-095 variant of `scripts/b2-metal-userdata.sh`: same QEMU build, same `cargo build --features b2-serror`, same `scripts/b2-kvm-smoke.py` with `CTOS_B2_TIMEOUT=90` and `CTOS_B2_STALL=15`, up to three attempts; plus the timer first, the `uname -srm` (no host name), KVM API line, `CTOSB2: result` line with commit and nonce, and the linger-then-poweroff.

## Consequences

- **If accepted and later enabled:** a re-check of the reduced profile can run on demand without a stored AWS key, with the role limited to one region, one instance type, one pinned image, tagged resources and no IAM, at roughly $0.03–0.07 per run [estimate].
- **Now:** nothing changes. The workflow is a file that fails closed. The honesty ledger gets a **Planned** row (documented sketch, not implemented), not Implemented.
- **Cost of the choice:** the console channel is narrow and text only; Spot capacity makes the check sometimes unavailable; the immutable-subject format and several account facts have to be verified by DSO before the first run.
- **Not decided here:** a schedule, a per-merge run, making it a required check, an S3 or SSM result channel, an on-demand fallback, a standalone sweeper. Each needs its own sponsor decision.

## Merge notes

This PR is **draft** and based on `main` at `3c47c0c` (after #151 merged; the first version of this branch was cut from `9a1c2bb` and rebased, with the append-only SUMMARY, moc and ledger conflicts resolved by keeping both sides). Open PRs #149 and #150 also append to `docs/SUMMARY.md`, `research/moc.md` and `docs/framework/honesty-ledger.md`; expect trivial append-only conflicts there after they merge. Resolve by keeping both sides. This PR does not touch those PRs.

## Reproducibility

```sh
# Syntax and lint (no AWS)
python3 -c "import yaml;yaml.safe_load(open('.github/workflows/kvm-recheck.yml'))"
actionlint .github/workflows/kvm-recheck.yml    # needs shellcheck on PATH
bash -n scripts/kvm-recheck-userdata.sh
sh scripts/check-public-ids.sh --self-test && sh scripts/check-public-ids.sh   # hits=0
```
