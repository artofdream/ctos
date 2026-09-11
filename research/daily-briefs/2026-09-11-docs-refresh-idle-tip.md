# Daily brief — 2026-09-11 (docs + second brain after #21–#28)

## Where we stopped

`main` ≈ `e80dc93` (Merge PR #28 / ADR-020). Docs-only refresh on `cursor/docs-refresh-post-27-192e` (rebased onto that tip). No kernel / `src/` / `linker.ld` / smoke-script behavior changes. This PR does **not** publish GitHub Pages.

Arc on `main`:

1. **#21** — multi-L3 layout + post-MMU frame init. Docker on `b2bbb99` **Failed**; Docker on `71ee15f` **Verified**.
2. **#23** — ASID isolation.
3. **#24** — standing EL0 + TTBR1 private-page first cut (ADR-016).
4. **#25** — EL1 fetch from TTBR1 high VA (ADR-017).
5. **#26** — identity-tear first cut (ADR-018). Docker **Verified** on `b0f0ee5`.
6. **#27** — high-VA jump + 16 KiB dedicated identity text range (ADR-019). First live-`.text` yank **Failed** (rustc `dyn Write` vtables). Docker **Verified** on `24d94e6`.
7. **#28** — high-VA vtable/fn-ptr rewrite + live identity `.text` tear (ADR-020). Serial `ident: reloc` / `ident: live`; `println!` after tear. **Verified**, not in-flight.

Sponsor cts-ai Docker **Verified** on `e80dc93`: 50 tests, `ident: reloc n=12`, live pages=37, force-fail ok. GHA merge-commit [34651404108](https://github.com/artofdream/ctos/actions/runs/34651404108) grepped the same class of markers. Keep the `24d94e6` Docker row.

Route 53 CNAME `ctos.artof.link` → `artofdream.github.io.` exists in zone `Z1178AFMV41RWP` (account `737290977112`). Custom-domain reachability stays **Planned** until a separate Pages PR. Do not claim https://ctos.artof.link works.

Optional Obsidian: structure only. `.obsidian/` / `.trash/` stay gitignored.

## Do next

1. Human or MRC review of this docs PR. Author does not merge (ADR-002).
2. Still **Planned**: `.rodata`/`.data`/heap tear, PAN on `cortex-a57`, umbrella EL0 isolation, Pages custom-domain reachability.
3. Do not claim “secure OS,” “the kernel moved,” or “EL0 isolated.” Do not switch `-cpu` to invent PAN.

## Honesty

- Docker `e80dc93` and `24d94e6` are sponsor probes. GHA on both merge commits was grepped here.
- Keep Failed history: `b2bbb99` Docker, live-`.text` yank, PR #27 `ac3ad0b` GHA.
- This session did not run QEMU on this VM.
