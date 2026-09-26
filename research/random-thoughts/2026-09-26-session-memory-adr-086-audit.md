# Session memory — ADR-086 audit (2026-09-26)

- The biggest surprise: “identity tears landed” hid **four** non-`_start` identity leftovers. The tears were section-driven (text / rodata / data / heap / pool), so ranges that belong to no section (the linker padding before `.data` @ `0x4020_1000`, the page at `__kernel_end`, the DTB hole) and the MMIO block were never anyone's job. A whole-table walker found them in one boot. Lesson: ratchet on *what is mapped*, not on *what we tore*.
- The padding tail is RO+X because the ADR-015 split marks `[KERNEL_TEXT, data)` as text. It is also in user TTBR0 for the same reason (`fill_user_map` uses the same predicate).
- ADR-055 §B row 2 literally demands yanking `_start`, which ADR-047 forbids. The row is unmeetable as written, so it needs a sponsor re-scope and must not be silently reinterpreted. The `-kernel` contract constrains the entry PA, not the post-boot TTBR0 mapping. That observation is for the sponsor to weigh (D2), not for me to act on.
- Taken SError: B1 is CI but a patched emulator behind a repo variable; B2-P is real hardware but one-off. ADR-060 says “accepted smoke machine”, so neither counts without a sponsor decision. I wrote that plainly.
- `el0: no data` in ADR-055 is stale wording; the smoke greps `el0: no kernel read`.
- `main` has no branch protection, so “CI-gated” is a convention.
