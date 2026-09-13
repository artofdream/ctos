# 2026-09-13 — Lower-EL FIQ while standing (ADR-043)

Shipped taken lower-EL FIQ while standing via temporary GICC FIQEn + SPSR F-clear probe (`el0: fiq`). SError has no safe inject on virt/cortex-a57 — honest `el0: serror-park`. Not “EL0 isolated.” PAN enable / yank `_start` / umbrella stay Planned. (evo-x2) intent; CloudAgent HELD — probe on agent box clone.
