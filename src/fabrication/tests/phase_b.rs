//! Phase B dispatch tests (baffle/support/conic-tolerance, V-8 wiring)
//! for `fabrication::compute`. See `../fixtures.rs` for shared fixtures.

use super::fixtures::*;
use crate::fabrication::compute;

/// V-8 acceptance: `fab_applicable` containing `"baffle"` on a
/// Cassegrain-family design with `field_diam_mm` present triggers
/// `baffle::compute`, populating `FabricationReport.baffle` with the A2 RC
/// worked-example numbers (B1 VERIFIED, [[v2-baffle-vignetting]]).
#[test]
fn compute_dispatches_baffle_for_cassegrain_family_with_field_diam() {
    let sheet = rc_sheet();
    let mut req = dummy_request();
    req.design = "rc".to_string();
    req.field_diam_mm = Some(20.0);
    let def = rc_def(Some(vec!["baffle".to_string()]));

    let report = compute(&sheet, &req, &def, &pyrex_catalog(), &enhanced_aluminum_catalog())
        .expect("compute must not error");

    let baffle = report.baffle.expect("baffle must be computed");
    assert_eq!(baffle.kind, "cassegrain");
    assert!((baffle.primary_baffle_hole_diameter_mm - 36.9).abs() < 0.1);
    assert!((baffle.secondary_baffle_tube_diameter_mm - 81.4).abs() < 0.1);
    assert!(baffle.vignetting_free);
}

/// `"baffle"` applicable but `field_diam_mm` absent -> `None` with a note
/// naming the missing field, not an error.
#[test]
fn compute_baffle_none_when_field_diam_mm_absent() {
    let sheet = rc_sheet();
    let mut req = dummy_request();
    req.design = "rc".to_string();
    let def = rc_def(Some(vec!["baffle".to_string()]));

    let report = compute(&sheet, &req, &def, &pyrex_catalog(), &enhanced_aluminum_catalog())
        .expect("compute must not error");

    assert!(report.baffle.is_none());
    assert!(report
        .notes
        .iter()
        .any(|n| n.contains("baffle") && n.contains("no 'field_diam_mm' given")));
}

/// `"baffle"` applicable but the design has no `SpecSheet.cassegrain`
/// (e.g. Newtonian) -> `None` with a note, never a panic (Newtonian
/// baffle variant scoped out, [[v2-baffle-vignetting]]).
#[test]
fn compute_baffle_none_for_non_cassegrain_design() {
    let sheet = newtonian_sheet();
    let mut req = dummy_request();
    req.field_diam_mm = Some(20.0);
    let def = newtonian_def(Some(vec!["baffle".to_string()]));

    let report = compute(&sheet, &req, &def, &pyrex_catalog(), &enhanced_aluminum_catalog())
        .expect("compute must not error");

    assert!(report.baffle.is_none());
    assert!(report.notes.iter().any(|n| n.contains("baffle") && n.contains("not applicable")));
}

/// V-8 acceptance: `fab_applicable` containing `"support"` with a
/// `substrate` name present triggers `support::compute`, defaulting to
/// `"9_point"`/9 when `support_scheme`/`support_n_points` are unset (S1
/// VERIFIED worked example, [[v2-support-seidel]]).
#[test]
fn compute_dispatches_support_with_defaults_when_scheme_unset() {
    let sheet = newtonian_sheet();
    let mut req = dummy_request();
    req.substrate = Some("pyrex".to_string());
    let def = newtonian_def(Some(vec!["support".to_string()]));

    let report = compute(&sheet, &req, &def, &pyrex_catalog(), &enhanced_aluminum_catalog())
        .expect("compute must not error");

    let support = report.support.expect("support must be computed");
    assert_eq!(support.support_scheme, "9_point");
    assert_eq!(support.n_points, 9);
}

/// `support_scheme`/`support_n_points` explicitly given override the
/// defaults.
#[test]
fn compute_dispatches_support_with_explicit_scheme() {
    let sheet = newtonian_sheet();
    let mut req = dummy_request();
    req.substrate = Some("pyrex".to_string());
    req.support_scheme = Some("3_point".to_string());
    req.support_n_points = Some(3);
    let def = newtonian_def(Some(vec!["support".to_string()]));

    let report = compute(&sheet, &req, &def, &pyrex_catalog(), &enhanced_aluminum_catalog())
        .expect("compute must not error");

    let support = report.support.expect("support must be computed");
    assert_eq!(support.support_scheme, "3_point");
    assert_eq!(support.n_points, 3);
}

/// `"support"` applicable but no `substrate` name given -> `None` with a
/// note, not an error (support-sag needs the substrate's density/E).
#[test]
fn compute_support_none_when_substrate_name_absent() {
    let sheet = newtonian_sheet();
    let req = dummy_request();
    let def = newtonian_def(Some(vec!["support".to_string()]));

    let report = compute(&sheet, &req, &def, &pyrex_catalog(), &enhanced_aluminum_catalog())
        .expect("compute must not error");

    assert!(report.support.is_none());
    assert!(report
        .notes
        .iter()
        .any(|n| n.contains("support") && n.contains("no 'substrate' name given")));
}

/// V-9 fix acceptance: for a Cassegrain-family design with conic
/// geometry, `tolerance` now carries the T1 VERIFIED conic-tolerance
/// fields (not just the Phase-A λ/8 surface figure), and the stale
/// "T1 pending" note text is gone.
#[test]
fn compute_dispatches_conic_tolerance_for_cassegrain_family() {
    let sheet = rc_sheet();
    let req = dummy_request();
    let def = rc_def(Some(vec!["tolerance".to_string()]));

    let report = compute(&sheet, &req, &def, &pyrex_catalog(), &enhanced_aluminum_catalog())
        .expect("compute must not error");

    let tolerance = report.tolerance.expect("tolerance must be computed");
    assert!((tolerance.primary_conic_tolerance - 0.03802).abs() < 1e-4);
    assert!(tolerance.secondary_conic_tolerance.is_some());
    assert_eq!(tolerance.conic_mode.as_deref(), Some("rc"));
    assert!(!report.notes.iter().any(|n| n.contains("T1) pending Phase B validation")));
}

/// A design with no `SpecSheet.cassegrain` (e.g. Newtonian) still gets
/// the Phase-A λ/8-only tolerance via `compute_phase_a` fallback — no
/// regression from the V-8 dispatch change.
#[test]
fn compute_tolerance_falls_back_to_phase_a_for_non_cassegrain_design() {
    let sheet = newtonian_sheet();
    let req = dummy_request();
    let def = newtonian_def(Some(vec!["tolerance".to_string()]));

    let report = compute(&sheet, &req, &def, &pyrex_catalog(), &enhanced_aluminum_catalog())
        .expect("compute must not error");

    let tolerance = report.tolerance.expect("tolerance must be computed");
    assert!((tolerance.surface_pv_tol_nm - 68.75).abs() < 1e-9);
    assert_eq!(tolerance.primary_conic_tolerance, 0.0);
}
