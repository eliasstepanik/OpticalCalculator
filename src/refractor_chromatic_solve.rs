//! `solve_powers` — N-element thin-lens chromatic system assembly.
//!
//! Sibling module to `refractor_chromatic` (which owns `solve_linear`); split
//! out to keep both files under the 300-line limit (constraints.md) while
//! staying flat (not a nested directory submodule) since the two functions
//! are simple, tightly-coupled peers rather than a deep hierarchy.

use crate::error::OptError;
use crate::glass::Glass;
use crate::refractor_chromatic::solve_linear;

/// Solve the N-element thin-lens chromatic system for element powers `φ_i`,
/// per architecture.md §3 ("General N-element thin-lens chromatic solver")
/// and formula-validation.md A3 (quintuplet DOF convention).
///
/// Assembles `M · φ = b` and delegates to [`solve_linear`]:
/// - Row 1 (always): `Σ φ_i = Φ` where `Φ = 1 / target_focal_length_mm`
///   (architecture.md §3 eq. 1 — total power).
/// - Row 2 (always): `Σ φ_i / V_i = 0` — first-order achromatism at C/F
///   (architecture.md §3 eq. 2).
/// - Row 3 (N ≥ 3 only): `Σ (P_i / V_i) φ_i = 0` — secondary-spectrum
///   control (architecture.md §3 eq. 3).
/// - Extra rows (N > 3 only, i.e. N=4/5 with more unknowns than the 3
///   physics rows above): per formula-validation.md A3, first add one row
///   `φ_i − φ_j = 0` for every duplicate-glass pair found by `Glass.name`
///   (duplicate glasses produce identical matrix columns in rows 1–3, so
///   the system is singular without an explicit equal-power constraint —
///   this is not a heuristic, it is required for M to be non-singular).
///   If DOF still remain after exhausting duplicates, add mirror-symmetric
///   pairing rows `φ_i − φ_(N+1−i) = 0` from the outside in (Kingslake's
///   lens-splitting principle — even power distribution minimizes
///   per-element strain). A caller-supplied `power_split` overrides rule 2
///   entirely: formula-validation.md A3 rule 3 specifies overridden rows as
///   `φ_i / w_i − φ_j / w_j = 0` for every adjacent pair `(i, i+1)`, i.e.
///   power distributed in fixed ratio `w_i` rather than symmetric/equal —
///   `power_split.len()` must equal N.
///
/// `ponytail:` for N > 3 the duplicate/mirror/power_split constraint rows
/// are a **documented predesign heuristic, not an optimized design** — real
/// quintuplet field-flattening/aberration balancing is out of scope
/// (architecture.md §3, §8 Q1; formula-validation.md A3). Callers presenting
/// N>3 results to the user should surface this caveat (Chunk 13's job, not
/// this function's).
///
/// Returns `Err(OptError::Unsolvable)` (with the offending glass names) if
/// `solve_linear` hits a singular/ill-conditioned system — e.g. two glasses
/// with identical Abbe number in a doublet, or an ordinary (non-ED) triplet
/// where row 3 is nearly a linear combination of rows 1–2.
pub fn solve_powers(
    target_focal_length_mm: f64,
    glasses: &[&Glass],
    power_split: Option<&[f64]>,
) -> Result<Vec<f64>, OptError> {
    let n = glasses.len();
    let names = || glasses.iter().map(|g| g.name.clone()).collect::<Vec<_>>();

    if n == 0 || target_focal_length_mm == 0.0 {
        return Err(OptError::Unsolvable(names()));
    }
    let phi_total = 1.0 / target_focal_length_mm;

    let mut matrix: Vec<Vec<f64>> = Vec::with_capacity(n);
    let mut rhs: Vec<f64> = Vec::with_capacity(n);

    // Row 1: total power, Σ φ_i = Φ.
    matrix.push(vec![1.0; n]);
    rhs.push(phi_total);

    // Row 2: achromatism, Σ φ_i / V_i = 0.
    matrix.push(glasses.iter().map(|g| 1.0 / g.vd).collect());
    rhs.push(0.0);

    // Row 3 (N >= 3): secondary spectrum, Σ (P_i / V_i) φ_i = 0.
    if n >= 3 {
        matrix.push(glasses.iter().map(|g| g.pgf / g.vd).collect());
        rhs.push(0.0);
    }

    // Extra rows (N > 3): close the system per formula-validation.md A3.
    if n > 3 {
        if let Some(weights) = power_split {
            if weights.len() != n {
                return Err(OptError::Unsolvable(names()));
            }
            // A3 rule 3: φ_i/w_i - φ_j/w_j = 0 for adjacent pairs, until
            // the system is square.
            for i in 0..(n - matrix.len()) {
                let mut row = vec![0.0; n];
                row[i] = 1.0 / weights[i];
                row[i + 1] = -1.0 / weights[i + 1];
                matrix.push(row);
                rhs.push(0.0);
            }
        } else {
            // A3 rule 1: equal-power rows for every duplicate-glass pair.
            let mut claimed = vec![false; n];
            for i in 0..n {
                if claimed[i] || matrix.len() >= n {
                    continue;
                }
                for j in (i + 1)..n {
                    if claimed[j] {
                        continue;
                    }
                    if glasses[i].name == glasses[j].name {
                        let mut row = vec![0.0; n];
                        row[i] = 1.0;
                        row[j] = -1.0;
                        matrix.push(row);
                        rhs.push(0.0);
                        claimed[j] = true;
                        if matrix.len() >= n {
                            break;
                        }
                    }
                }
            }

            // A3 rule 2: mirror-symmetric pairing from the outside in for
            // any remaining DOF.
            let mut lo = 0usize;
            let mut hi = n.saturating_sub(1);
            while matrix.len() < n && lo < hi {
                let mut row = vec![0.0; n];
                row[lo] = 1.0;
                row[hi] = -1.0;
                matrix.push(row);
                rhs.push(0.0);
                lo += 1;
                hi -= 1;
            }
        }
    }

    if matrix.len() != n {
        return Err(OptError::Unsolvable(names()));
    }

    solve_linear(matrix, rhs).map_err(|_| OptError::Unsolvable(names()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn glass(name: &str, vd: f64, pgf: f64) -> Glass {
        Glass {
            name: name.to_string(),
            nd: 1.5, // unused by solve_powers; radii conversion is Chunk 13's job
            vd,
            pgf,
        }
    }

    #[test]
    fn doublet_matches_bk7_f2_closed_form() {
        // formula-validation.md "Chunk 10 (solve_powers) — load-bearing
        // doublet reference": BK7 (Vd 64.17) / F2 (Vd 36.37), f = 1000mm.
        let bk7 = glass("BK7", 64.17, 0.5349);
        let f2 = glass("F2", 36.37, 0.5834);
        let glasses = [&bk7, &f2];

        let phi = solve_powers(1000.0, &glasses, None).expect("doublet must solve");

        assert!((phi[0] - 2.3082734e-3).abs() < 1e-9, "phi1 = {}", phi[0]);
        assert!((phi[1] - (-1.3082734e-3)).abs() < 1e-9, "phi2 = {}", phi[1]);

        // Row (1)/(2) residuals near zero.
        let row1: f64 = phi.iter().sum::<f64>() - 1e-3;
        let row2: f64 = phi[0] / bk7.vd + phi[1] / f2.vd;
        assert!(row1.abs() < 1e-12, "row1 residual = {row1}");
        assert!(row2.abs() < 1e-12, "row2 residual = {row2}");
    }

    #[test]
    fn ed_triplet_solves_well_conditioned_vs_ordinary_triplet() {
        // formula-validation.md F1 fix: a real ED-vs-ordinary comparison,
        // not a vacuous "row-3 residual ~ 0" check (which is trivially true
        // once row 3 is part of the solved system for ANY glass combo).
        let fpl53 = glass("FPL53", 94.66, 0.5340);
        let bk7 = glass("BK7", 64.17, 0.5349);
        let f2 = glass("F2", 36.37, 0.5834);
        let ed_glasses = [&fpl53, &bk7, &f2];

        let ed_phi = solve_powers(1000.0, &ed_glasses, None).expect("ED triplet must solve");
        for (i, p) in ed_phi.iter().enumerate() {
            assert!(p.is_finite(), "ED triplet phi[{i}] not finite: {p}");
        }
        // Row residuals all near zero (the system was solved, not just
        // algebraically present).
        let row1: f64 = ed_phi.iter().sum::<f64>() - 1e-3;
        let row2: f64 = ed_phi[0] / fpl53.vd + ed_phi[1] / bk7.vd + ed_phi[2] / f2.vd;
        let row3: f64 = ed_phi[0] * fpl53.pgf / fpl53.vd
            + ed_phi[1] * bk7.pgf / bk7.vd
            + ed_phi[2] * f2.pgf / f2.vd;
        assert!(row1.abs() < 1e-9, "row1 residual = {row1}");
        assert!(row2.abs() < 1e-9, "row2 residual = {row2}");
        assert!(row3.abs() < 1e-9, "row3 residual = {row3}");

        // The meaningful comparison: this triplet's secondary-spectrum
        // residual is exactly 0 by construction of row 3, whereas the plain
        // BK7/F2 doublet (no row 3, no ED glass) leaves Sigma(P/V)*phi =
        // -1.7446e-6 nonzero (formula-validation.md worked value) — proving
        // row 3 is what an ED element buys you, not present-but-vacuous.
        let doublet_secondary_spectrum = -1.7446e-6_f64;
        assert!(
            row3.abs() < doublet_secondary_spectrum.abs(),
            "ED triplet row3 residual ({row3}) should be far smaller than \
             the doublet's uncorrected secondary spectrum ({doublet_secondary_spectrum})"
        );

        // Ordinary triplet: three non-ED glasses with close Abbe numbers
        // (BK7 64.17, K5 59.48, SK5 61.15 — all ordinary crowns) makes row 3
        // nearly a linear combination of rows 1-2 -> ill-conditioned. Real
        // catalog glasses per configs/glasses.toml.
        let k5 = glass("K5", 59.48, 0.5423);
        let sk5 = glass("SK5", 61.15, 0.5410);
        let ordinary_glasses = [&bk7, &k5, &sk5];

        match solve_powers(1000.0, &ordinary_glasses, None) {
            Err(OptError::Unsolvable(_)) => {
                // Explicit singular detection — acceptable failure signature.
            }
            Ok(ordinary_phi) => {
                // Otherwise must be ill-conditioned: |phi| an order of
                // magnitude larger than the well-conditioned ED triplet's.
                let ed_max = ed_phi.iter().fold(0.0_f64, |m, p| m.max(p.abs()));
                let ordinary_max = ordinary_phi.iter().fold(0.0_f64, |m, p| m.max(p.abs()));
                assert!(
                    ordinary_max > ed_max * 10.0,
                    "ordinary triplet not ill-conditioned: ordinary_max={ordinary_max}, \
                     ed_max={ed_max}"
                );
            }
            Err(other) => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn quintuplet_duplicate_glass_rows_force_equal_power() {
        // formula-validation.md A3: shipped design glass list
        // [FPL53, FPL53, BK7, F2, BK7] — duplicates of FPL53 (idx 0,1) and
        // BK7 (idx 2,4) consume both extra DOF via equal-power rows.
        let fpl53_a = glass("FPL53", 94.66, 0.5340);
        let fpl53_b = glass("FPL53", 94.66, 0.5340);
        let bk7_a = glass("BK7", 64.17, 0.5349);
        let f2 = glass("F2", 36.37, 0.5834);
        let bk7_b = glass("BK7", 64.17, 0.5349);
        let glasses = [&fpl53_a, &fpl53_b, &bk7_a, &f2, &bk7_b];

        let phi = solve_powers(1000.0, &glasses, None).expect("quintuplet must solve");

        assert_eq!(phi.len(), 5);
        for (i, p) in phi.iter().enumerate() {
            assert!(p.is_finite(), "quintuplet phi[{i}] not finite: {p}");
        }

        // Duplicate-glass constraint rows must force equal power exactly
        // (within solver tolerance), not merely "some finite answer".
        assert!(
            (phi[0] - phi[1]).abs() < 1e-9,
            "FPL53 elements should have equal power: phi0={}, phi1={}",
            phi[0],
            phi[1]
        );
        assert!(
            (phi[2] - phi[4]).abs() < 1e-9,
            "BK7 elements should have equal power: phi2={}, phi4={}",
            phi[2],
            phi[4]
        );
    }
}
