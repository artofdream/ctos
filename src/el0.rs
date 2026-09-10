//! EL0 isolation is **Planned** (P-SEC-3 / ADR-013).
//!
//! This module does not drop to EL0, does not install a lower-EL map,
//! and does not claim isolation. The closing probe (lower-EL cannot
//! execute kernel data) is not built. See `docs/framework/el0.md`.

/// Always false until a later ADR actually enters EL0.
#[allow(dead_code)] // hello build has no caller; `#[test_case]` does.
pub fn is_active() -> bool {
    false
}

#[cfg(test)]
#[test_case]
fn el0_is_not_active() {
    assert!(
        !is_active(),
        "EL0 must stay off until ADR-013's later implementation"
    );
}
