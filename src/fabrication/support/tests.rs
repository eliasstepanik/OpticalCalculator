//! `#[cfg(test)] mod tests` body for `fabrication/support.rs`.

use super::*;

/// Real Pyrex catalog entry matching `configs/substrates.toml`'s `pyrex`
/// entry (density 2.23 g/cm³, E = 63 GPa) exactly.
fn pyrex() -> Substrate {
    Substrate {
        name: "pyrex".to_string(),
        cte_ppm_per_k: 3.25,
        density_g_cm3: 2.23,
        cost_tier: 2,
        max_aperture_mm: 500.0,
        youngs_modulus_gpa: 63.0,
    }
}

/// Worked example ([[formula-validation-v2]] S1 VERIFIED): 200mm Pyrex
/// mirror, 30mm thick, 9-point back support -> 0.372 nm (±0.01nm), which
/// is ≈0.54% of the λ/8 (68.75nm) budget.
#[test]
fn worked_example_200mm_pyrex_9_point_support() {
    let spec = compute(200.0, 30.0, &pyrex(), "9_point", 9).expect("9_point is a valid scheme");

    assert!((spec.sag_nm - 0.372).abs() < 0.01);
    assert!((spec.sag_pct_of_lambda_8 - 0.5404153706375932).abs() < 1e-6);
    assert_eq!(spec.support_scheme, "9_point");
    assert_eq!(spec.n_points, 9);
    assert!(!spec.notes.is_empty());
    // 0.372nm is far below the 137.5nm λ/4 reference budget.
    assert!(spec.within_quarter_wave);
}

/// V-10: `full_disk` reports sag as exactly `0.0` ("negligible", per
/// [[v2-support-seidel]]) regardless of `n_points`, and is always within
/// the λ/4 budget.
#[test]
fn full_disk_scheme_reports_zero_sag() {
    let spec = compute(200.0, 30.0, &pyrex(), "full_disk", 0).expect("full_disk is a valid scheme");

    assert_eq!(spec.sag_nm, 0.0);
    assert_eq!(spec.sag_pct_of_lambda_8, 0.0);
    assert!(spec.within_quarter_wave);
    assert!(spec.notes.iter().any(|n| n.contains("negligible (full-disk support)")));
}

/// V-10: `whiffletree` is modeled as its effective support-point count
/// (physical pivots / 2) — an 18-point whiffletree should give the exact
/// same sag as direct 9-point support (same C, same effective N).
#[test]
fn whiffletree_18_point_matches_effective_9_point_sag() {
    let whiffletree = compute(200.0, 30.0, &pyrex(), "whiffletree", 18)
        .expect("whiffletree is a valid scheme");
    let nine_point =
        compute(200.0, 30.0, &pyrex(), "9_point", 9).expect("9_point is a valid scheme");

    assert!((whiffletree.sag_nm - nine_point.sag_nm).abs() < 1e-9);
    assert!(whiffletree
        .notes
        .iter()
        .any(|n| n.contains("effective support-point count (9)")));
    // n_points on the spec still reports the physical pivot count given.
    assert_eq!(whiffletree.n_points, 18);
}

/// `within_quarter_wave` is `false` for a sag that exceeds the fixed
/// 137.5nm (550nm λ/4) reference budget — a deliberately thin/sparse
/// mirror (3-point support, thin mirror) to push sag well past budget.
#[test]
fn within_quarter_wave_false_when_sag_exceeds_budget() {
    let spec = compute(600.0, 20.0, &pyrex(), "3_point", 3).expect("3_point is a valid scheme");

    assert!(spec.sag_nm > 137.5, "expected sag to exceed budget, got {}", spec.sag_nm);
    assert!(!spec.within_quarter_wave);
}

/// Same mirror, 3-point support -> ≈6.69 nm (±0.05nm), ≈9.7% of the λ/8
/// budget — an order of magnitude worse than 9-point, per the S1 doc.
#[test]
fn worked_example_200mm_pyrex_3_point_support() {
    let spec = compute(200.0, 30.0, &pyrex(), "3_point", 3).expect("3_point is a valid scheme");

    assert!((spec.sag_nm - 6.69).abs() < 0.05);
    assert!((spec.sag_pct_of_lambda_8 - 9.727476671476674).abs() < 1e-6);
}

/// 6-point back support shares 9-point's `C = 0.04875` baseline constant
/// (S1 VERIFIED table) — for the SAME N, sag should match the 9-point
/// formula's per-N-point scaling exactly (same C, different N in the
/// N² divisor only).
#[test]
fn six_point_and_nine_point_share_same_c_constant() {
    let six = support_sag_nm(200.0, 30.0, 2230.0, 63.0, 6, 0.04875);
    let nine = support_sag_nm(200.0, 30.0, 2230.0, 63.0, 9, 0.04875);

    // 6-point has fewer support points than 9-point with the same C, so
    // its sag must be strictly larger (N² divisor is smaller).
    assert!(six > nine);
    // Ratio should be exactly (9/6)^2 = 2.25 given identical C.
    assert!((six / nine - 2.25).abs() < 1e-9);
}

/// Unrecognized scheme name returns `OptError::InvalidValue`, not a
/// panic.
#[test]
fn unrecognized_support_scheme_returns_invalid_value_error() {
    let err = compute(200.0, 30.0, &pyrex(), "whiffletree_42", 42)
        .expect_err("unrecognized scheme must error, not panic");

    match err {
        OptError::InvalidValue { field, reason } => {
            assert_eq!(field, "support_scheme");
            assert!(reason.contains("whiffletree_42"));
        }
        other => panic!("expected InvalidValue, got {other:?}"),
    }
}

/// Dimensional/scaling sanity: doubling thickness `t` divides sag by 4
/// (t² in the denominator), holding all else fixed.
#[test]
fn sag_scales_as_inverse_thickness_squared() {
    let base = support_sag_nm(200.0, 30.0, 2230.0, 63.0, 9, 0.04875);
    let doubled_t = support_sag_nm(200.0, 60.0, 2230.0, 63.0, 9, 0.04875);

    assert!((base / doubled_t - 4.0).abs() < 1e-9);
}

/// Dimensional/scaling sanity: doubling diameter `D` multiplies sag by
/// 16 (D⁴ in the numerator), holding all else fixed.
#[test]
fn sag_scales_as_diameter_to_the_fourth() {
    let base = support_sag_nm(200.0, 30.0, 2230.0, 63.0, 9, 0.04875);
    let doubled_d = support_sag_nm(400.0, 30.0, 2230.0, 63.0, 9, 0.04875);

    assert!((doubled_d / base - 16.0).abs() < 1e-6);
}

/// Alternate scheme-name spellings (hyphen vs underscore, plus the
/// "edge_triangle" alias for 3-point) all resolve to the same constants.
#[test]
fn scheme_name_aliases_resolve_to_same_constant() {
    assert_eq!(scheme_constant("3_point").unwrap(), scheme_constant("3-point").unwrap());
    assert_eq!(
        scheme_constant("3_point").unwrap(),
        scheme_constant("edge_triangle").unwrap()
    );
    assert_eq!(scheme_constant("6_point").unwrap(), scheme_constant("6-point").unwrap());
    assert_eq!(scheme_constant("9_point").unwrap(), scheme_constant("6_point").unwrap());
}
