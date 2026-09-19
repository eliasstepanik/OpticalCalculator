//! Coddington shape-factor radii-from-power conversion for the refractor
//! pipeline (`designs/refractor.rs`, Chunk 13).
//!
//! Sibling module to `refractor.rs`, split out per the file's own
//! documented rationale (mirrors the `refractor_chromatic` /
//! `refractor_chromatic_solve` split, Chunk 10) to keep both files under
//! the 300-line limit (constraints.md) while staying flat.
//!
//! Reference: A.E. Conrady's / R. Kingslake's thin-lens predesign shape
//! factor (Kingslake, *Lens Design Fundamentals*, Academic Press) — for a
//! thin lens with a free front surface and an object at infinity, the
//! **Coddington shape factor** `q = (R2+R1)/(R2−R1)` that minimizes
//! third-order spherical aberration is the closed form
//! `q_min = 2(n²−1)/(n+2)`. Combined with the lensmaker's equation
//! `φ = (n−1)(1/R1 − 1/R2)`, solving for the two radii at fixed target
//! power `φ` and shape `q_min` gives:
//!
//! ```text
//! R1 = 2·f·(n−1) / (q_min + 1)
//! R2 = 2·f·(n−1) / (q_min − 1)      where f = 1/φ
//! ```
//!
//! formula-validation.md's Chunk 13 worked example validates this exact
//! form: BK7 (n=1.51680), φ1=+2.3082734e-3 mm⁻¹ (f1=433.22mm) → q=0.7397,
//! R1=+257.4mm, R2=−1720.2mm (±0.5mm). The sign of `f` naturally yields the
//! "equiconvex crown" shape for a positive-power element and the
//! "equiconcave flint" shape for a negative-power element (decision #1's
//! symmetric-shape-default convention) — no per-sign branching needed, the
//! same formula produces both.
//!
//! For every element after the first in a cemented in-contact stack
//! (architecture.md §3 models the whole system as "a thin, in-contact
//! system of N elements"), the front surface `R1` is fixed by the cemented
//! contact with the previous element's `R2` — it is not a free shape
//! choice. [`cemented_r2`] solves the *remaining* free surface `R2` from
//! the element's own power equation with `R1` held fixed:
//! `1/R2 = 1/R1 − φ/(n−1)`. formula-validation.md validates this too: the
//! cemented F2 flint (R1=−1720.2mm forced from the BK7 crown, φ2=
//! −1.3082734e-3 mm⁻¹) solves to R2=+654.2mm (±0.5mm).

/// Below this magnitude a power is treated as exactly zero (a flat/afocal
/// surface) to avoid a division by zero — never NaN/Inf from ordinary use,
/// only from a genuinely zero-power element.
const POWER_EPSILON: f64 = 1e-12;

/// Coddington minimum-spherical-aberration shape factor `q = 2(n²−1)/(n+2)`
/// (Kingslake). Depends only on refractive index.
pub fn coddington_q(nd: f64) -> f64 {
    2.0 * (nd * nd - 1.0) / (nd + 2.0)
}

/// Radii for a leading element whose front surface is free (both `R1` and
/// `R2` chosen via the Coddington minimum-SA shape). Returns
/// `(f64::INFINITY, f64::INFINITY)` for a ~zero-power element (flat, no
/// panic/NaN) rather than dividing by zero.
pub fn coddington_radii(power_mm_inv: f64, nd: f64) -> (f64, f64) {
    if power_mm_inv.abs() < POWER_EPSILON {
        return (f64::INFINITY, f64::INFINITY);
    }
    let f_mm = 1.0 / power_mm_inv;
    let q = coddington_q(nd);
    let r1 = 2.0 * f_mm * (nd - 1.0) / (q + 1.0);
    let r2 = 2.0 * f_mm * (nd - 1.0) / (q - 1.0);
    (r1, r2)
}

/// Solve `R2` for an element whose `R1` is fixed by a cemented contact with
/// the previous element's `R2`, from `φ = (n−1)(1/R1 − 1/R2)` rearranged to
/// `1/R2 = 1/R1 − φ/(n−1)`. Handles a flat forced front surface
/// (`r1_forced_mm.is_infinite()`, i.e. `1/R1 = 0`) and a ~zero resulting
/// power (flat `R2`) without ever dividing by zero into NaN/Inf-by-accident
/// — both degenerate cases return `f64::INFINITY` explicitly.
pub fn cemented_r2(power_mm_inv: f64, nd: f64, r1_forced_mm: f64) -> f64 {
    let inv_r1 = if r1_forced_mm.is_infinite() {
        0.0
    } else {
        1.0 / r1_forced_mm
    };
    let inv_r2 = inv_r1 - power_mm_inv / (nd - 1.0);
    if inv_r2.abs() < POWER_EPSILON {
        f64::INFINITY
    } else {
        1.0 / inv_r2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// formula-validation.md Chunk 13: BK7 φ1=+2.3082734e-3, f1=433.22mm ->
    /// q=0.7397, R1=+257.4mm, R2=-1720.2mm (+/-0.5mm each).
    #[test]
    fn coddington_radii_match_bk7_worked_example() {
        let n_bk7 = 1.51680;
        let phi1 = 2.3082734e-3;

        let q = coddington_q(n_bk7);
        assert!((q - 0.7397).abs() < 0.0005, "q = {q}");

        let (r1, r2) = coddington_radii(phi1, n_bk7);
        assert!((r1 - 257.4).abs() < 0.5, "R1 = {r1}");
        assert!((r2 - (-1720.2)).abs() < 0.5, "R2 = {r2}");
    }

    /// formula-validation.md Chunk 13: cemented F2 flint, R1 forced to the
    /// BK7 crown's R2 (-1720.2mm), phi2=-1.3082734e-3 -> R2=+654.2mm.
    #[test]
    fn cemented_r2_matches_f2_worked_example() {
        let n_f2 = 1.62004;
        let phi2 = -1.3082734e-3;
        let r1_forced = -1720.2246433747036; // BK7 crown's exact R2

        let r2 = cemented_r2(phi2, n_f2, r1_forced);
        assert!((r2 - 654.2).abs() < 0.5, "R2 = {r2}");
    }

    /// A ~zero-power leading element must return flat radii, not NaN/panic.
    #[test]
    fn coddington_radii_zero_power_returns_infinite_flat() {
        let (r1, r2) = coddington_radii(0.0, 1.5);
        assert!(r1.is_infinite() && r2.is_infinite());
    }

    /// A ~zero-power cemented element with a finite forced R1 must return
    /// an infinite (flat) R2, not divide-by-zero garbage.
    #[test]
    fn cemented_r2_zero_power_with_finite_r1_is_consistent() {
        // phi = 0 means 1/R2 = 1/R1 exactly -> R2 = R1 (not flat, unless
        // R1 itself is flat). Use R1 flat to hit the true degenerate case.
        let r2 = cemented_r2(0.0, 1.5, f64::INFINITY);
        assert!(r2.is_infinite());
    }
}
