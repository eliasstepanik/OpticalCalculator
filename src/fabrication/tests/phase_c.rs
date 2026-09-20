//! Phase C dispatch tests (collimation/seidel, VC3 wiring) for
//! `fabrication::compute`. See `../fixtures.rs` for shared fixtures.

use super::fixtures::*;
use crate::fabrication::compute;

/// VC3 acceptance: `fab_applicable` containing `"collimation"` on a
/// Cassegrain-family design triggers `collimation::compute`, populating
/// `FabricationReport.collimation` with the A2 RC worked-example numbers
/// (C1 VERIFIED, [[v2-tolerance-collimation]]).
#[test]
fn compute_dispatches_collimation_for_cassegrain_family() {
    let sheet = rc_sheet();
    let req = dummy_request();
    let def = rc_def(Some(vec!["collimation".to_string()]));

    let report = compute(&sheet, &req, &def, &pyrex_catalog(), &enhanced_aluminum_catalog())
        .expect("compute must not error");

    let collimation = report.collimation.expect("collimation must be computed");
    assert!((collimation.coma_per_mm_decenter_waves - 2.257).abs() < 0.01);
    assert!((collimation.allowable_decenter_mm - 0.111).abs() < 0.001);
    assert!((collimation.allowable_tilt_arcsec - 139.0).abs() < 1.0);
}

/// `"collimation"` applicable but the design has no `SpecSheet.cassegrain`
/// (e.g. Newtonian) -> `None` with a note, never a panic (two-mirror-only
/// subsystem).
#[test]
fn compute_collimation_none_for_non_cassegrain_design() {
    let sheet = newtonian_sheet();
    let req = dummy_request();
    let def = newtonian_def(Some(vec!["collimation".to_string()]));

    let report = compute(&sheet, &req, &def, &pyrex_catalog(), &enhanced_aluminum_catalog())
        .expect("compute must not error");

    assert!(report.collimation.is_none());
    assert!(report
        .notes
        .iter()
        .any(|n| n.contains("collimation") && n.contains("not applicable")));
}

/// `"collimation"` excluded from `fab_applicable` -> `None` with a note,
/// same "not applicable" pattern as every other subsystem.
#[test]
fn compute_collimation_none_when_not_in_fab_applicable() {
    let sheet = rc_sheet();
    let req = dummy_request();
    let def = rc_def(Some(vec!["substrate".to_string()]));

    let report = compute(&sheet, &req, &def, &pyrex_catalog(), &enhanced_aluminum_catalog())
        .expect("compute must not error");

    assert!(report.collimation.is_none());
    assert!(report
        .notes
        .iter()
        .any(|n| n.contains("collimation") && n.contains("not applicable")));
}

/// VC3 acceptance: `fab_applicable` containing `"seidel"` on a
/// Cassegrain-family design WITH `field_diam_mm` present triggers
/// `seidel::compute`, populating `FabricationReport.seidel` with the RC
/// aplanatic (S_II≈0) headline (S2 VERIFIED, [[v2-support-seidel]]).
#[test]
fn compute_dispatches_seidel_for_cassegrain_family_with_field_diam() {
    let sheet = rc_sheet();
    let mut req = dummy_request();
    req.field_diam_mm = Some(20.0);
    let def = rc_def(Some(vec!["seidel".to_string()]));

    let report = compute(&sheet, &req, &def, &pyrex_catalog(), &enhanced_aluminum_catalog())
        .expect("compute must not error");

    let seidel = report.seidel.expect("seidel must be computed");
    assert!(seidel.coma_s2.abs() < 1e-6, "got {}", seidel.coma_s2);
    assert_eq!(seidel.conic_mode.as_deref(), Some("aplanatic (coma-nulled)"));
    assert!(!seidel.provisional);
}

/// `"seidel"` applicable but `field_diam_mm` absent -> `None` with a note
/// naming the missing field, not an error (same precondition pattern as
/// `"baffle"`).
#[test]
fn compute_seidel_none_when_field_diam_mm_absent() {
    let sheet = rc_sheet();
    let req = dummy_request();
    let def = rc_def(Some(vec!["seidel".to_string()]));

    let report = compute(&sheet, &req, &def, &pyrex_catalog(), &enhanced_aluminum_catalog())
        .expect("compute must not error");

    assert!(report.seidel.is_none());
    assert!(report
        .notes
        .iter()
        .any(|n| n.contains("seidel") && n.contains("no 'field_diam_mm' given")));
}

/// `"seidel"` applicable but the design has no `SpecSheet.cassegrain`
/// (e.g. Newtonian) -> `None` with a note, never a panic.
#[test]
fn compute_seidel_none_for_non_cassegrain_design() {
    let sheet = newtonian_sheet();
    let mut req = dummy_request();
    req.field_diam_mm = Some(20.0);
    let def = newtonian_def(Some(vec!["seidel".to_string()]));

    let report = compute(&sheet, &req, &def, &pyrex_catalog(), &enhanced_aluminum_catalog())
        .expect("compute must not error");

    assert!(report.seidel.is_none());
    assert!(report
        .notes
        .iter()
        .any(|n| n.contains("seidel") && n.contains("not applicable")));
}
