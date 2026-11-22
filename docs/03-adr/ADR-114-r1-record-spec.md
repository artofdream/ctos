# ADR-114 R1 — Record format, golden files and rules (spec only)

- Status: **Spec (Proposed).** This page fixes the R1 input format, transform, reject rules and golden files for the [ADR-114](ADR-114-first-target-workload-record-processing.md) first target workload. It changes no claim. There is no app, no marker, no kernel or `libctos` change and **no measurement** here; those are R2+. Do not self-merge ([ADR-002](ADR-002-pr-identity-split.md)).
- Date: 2026-11-22 (CET)
- Base: main `e35ab0fd` (after [#178](https://github.com/artofdream/ctos/pull/178), ADR-114 merged as Proposed).
- Machine: on-prem amd64 host, throwaway clone. Docs + fixtures + a host-side Python oracle only. No cloud, no CloudAgent, no `kvm-recheck` dispatch.
- Asked by: the user, 2026-11-22 ("proceed with R1").
- Files: [`tests/fixtures/adr114/workin`](https://github.com/artofdream/ctos/blob/main/tests/fixtures/adr114/workin) (golden input), [`tests/fixtures/adr114/workout`](https://github.com/artofdream/ctos/blob/main/tests/fixtures/adr114/workout) (golden expected output), [`scripts/adr114-golden.py`](https://github.com/artofdream/ctos/blob/main/scripts/adr114-golden.py) (host-side oracle).

ctos stays a research / teaching kernel: not production, not a "secure OS", not POSIX. Passing this spec on QEMU would say nothing about real hardware.

## Fit with today's limits (read from `main`)

| Limit on `main` | Value | How R1 fits |
| --- | --- | --- |
| `vfs::FILE_MAX` | 1024 bytes | Golden input is **505 bytes**, golden output **447 bytes**. Both must stay ≤ 1024. |
| `FS_IO_MAX` (per `fs_read` / `fs_write`) | 64 bytes | The app reads and writes in chunks of at most 64 bytes. Records may straddle a chunk boundary; the golden input has six that do (lines 8, 16, 24, 31, 37, 47) and one line (37) longer than a whole 64-byte read. |
| FAT16 path grammar | flat `/name`, 1–8 of `[a-z0-9_-]`, no extension, `PATH_MAX` 16 | Input path **`/workin`** (8.3 `WORKIN`), output path **`/workout`** (8.3 `WORKOUT`). |
| Open handles | 4 FAT | One input + one output. |
| EL0 memory | 8 load pages + 1 stack page, no heap | The rules below need one line buffer of 21 bytes, two counters and one 64-byte I/O buffer each way. No allocation. |
| No EL0 `unlink` / `truncate` / `rename` / `fsync` | — | The R1 golden run assumes `/workout` does **not** exist before the run. What happens when it does is an R2 gap, not defined here. |

## Input format

- The input is a byte string of length 1–1024. It is split into **lines** at each `\n` (0x0A). The `\n` is not part of the line.
- Bytes after the last `\n` (if any) form a **truncated final line**. It is always rejected, even if its text would be a valid record.
- A line is a **valid record** if and only if it matches, byte for byte:

```text
record = id "," value
id     = 1*8 DIGIT                 ; leading zeros allowed, kept verbatim
value  = [ "-" ] 1*10 DIGIT        ; leading zeros allowed
DIGIT  = %x30-39                   ; ASCII '0'..'9' only
```

  and the value, read as a signed decimal, is in the `i32` range −2147483648 … 2147483647.
- Every other line is a **rejected record**. That includes: an empty line; `\r` (so CRLF line endings reject every line); spaces or tabs anywhere; `+`; a NUL or any non-ASCII byte; an id of 0 or more than 8 digits; a negative id; an empty or sign-only value; more than 10 value digits; `i32` overflow either way; `--`; hex such as `0x10`; a missing comma or more than one comma.
- The longest valid line is 20 bytes (8 + 1 + 11). An app may stop buffering a line once it passes 20 bytes, keep reading to the next `\n`, and count it rejected. A rejected line must never crash the app or the kernel.
- There are **no cross-record rules** in R1. Duplicate ids are each processed on their own (the golden input has id `2` twice).

## Transform (per valid record, in input order)

1. **Canonical value**: the parsed `i32` in decimal, no leading zeros, a leading `-` only if negative. `-0` and `-000` become `0`; `007` becomes `7`.
2. **Body**: `id` (verbatim, leading zeros kept) `,` canonical value.
3. **Checksum**: CRC-16/CCITT-FALSE over the body bytes — polynomial 0x1021, initial value 0xFFFF, no input or output reflection, final XOR 0. Check value: `"123456789"` → `0x29b1`. Written as exactly 4 lowercase hex digits, zero-padded.
4. **Output line**: body `,` checksum `\n`.

## Output format

```text
output  = *( id "," canonical-value "," 4HEXLOWER LF ) trailer
trailer = "ok=" count " rejected=" count LF   ; decimal, no leading zeros
```

- `ok` is the number of valid records, `rejected` the number of rejected lines (truncated final line included). Every line of the input is counted exactly once.
- The output is **complete if and only if** it ends with a trailer whose `ok` equals the number of result lines before it. A reader (the smoke, or a later restart path in R4) must treat any other `/workout` as incomplete, never as a result. This is the hook R3 (interrupted write) and R4 (restart) build on; R1 does not claim either.
- The whole output must be ≤ 1024 bytes (`FILE_MAX`).

## Whole-run failures

These end the run with a nonzero exit and no complete output. Marker shape is the one sketched in ADR-114: `work: fail reason=<token>`.

| Token | When |
| --- | --- |
| `input_missing` | `/workin` cannot be opened. |
| `input_empty` | `/workin` has 0 bytes. |
| `input_too_big` | `/workin` is longer than 1024 bytes (cannot be built on today's FAT16; kept so the rule survives a larger `FILE_MAX`). |
| `output_too_big` | The output would exceed 1024 bytes (possible: 256 four-byte records give 2,304 bytes of results). |
| `io` | `fs_create` / `fs_write` / `fs_read` / `fs_close` returned an error. |

On success the app exits 0, and the planned marker is `work: ok records=<ok> rejected=<rejected> ticks=<t>`. Bad records are **not** a whole-run failure; they only raise `rejected`.

A **virtual SError** taken during a run is not a reject and not one of these tokens: it keeps the existing fail-closed path ([ADR-053](ADR-053-taken-serror-hard-stop.md) / [ADR-112](ADR-112-b3-graviton-real-hardware-serror-plan.md)). No survival claim.

## Golden files

- `tests/fixtures/adr114/workin`: 55 lines, 505 bytes — 33 valid records (incl. `i32` min and max, leading-zero id and value, `-0`, a duplicate id) and 22 rejects (every class listed above except tabs, including the CRLF line, a NUL line, a 73-byte line, and a truncated final line `33,7` with no `\n`).
- `tests/fixtures/adr114/workout`: 34 lines, 447 bytes, trailer `ok=33 rejected=22`.
- Both are byte-exact (`.gitattributes` marks them `-text`, so no line-ending rewrite).
- `python3 scripts/adr114-golden.py --check` recomputes `workout` from `workin` with this spec and fails on any byte difference; `--self-test` checks the CRC catalogue value and each reject / failure class on small inputs. It is a spec oracle on the host, not the EL0 app, and it is not wired into a required check by this PR.

## Not in R1 (left on purpose)

- **Performance**: not measured. ADR-114 says measure first (CNTPCT ticks, ADR-046 method) once the app exists; the pass/fail threshold is *TBD-by-sponsor* after at least one baseline. TCG timings are not hardware timings.
- **Larger data**: needs the `FILE_MAX` gap closed (R2) and a sponsor-set size.
- **App code, `libctos` ops, smoke markers**: R2, one finding per PR.
- **Negative smokes** (malformed file, memory exhaustion, app fault, interrupted write, **virtual SError**): R3. **Soak, limits, diagnostics, restart**: R4.
- The ledger row for ADR-114 stays **Planned**. Nothing flips until probes are Verified on CI and the sponsor accepts the wording.
