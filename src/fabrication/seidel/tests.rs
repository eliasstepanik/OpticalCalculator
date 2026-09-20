//! Tests for `seidel::compute` — the mandatory S2 self-check
//! ([[formula-validation-v2]] item S2, [[v2-support-seidel]] "Worked
//! self-check") reproduced numerically: RC S_II ≈ 0 (aplanatic), classical
//! Cassegrain S_II clearly non-zero, and RC's edge blur smaller than
//! classical's at the same field (the core physics validation).

use super::*;

/// Common A2 RC-geometry inputs (also used by v1's baffle/collimation/
/// support worked examples): D=200, F=1600, R1=1200, R2=654.5455,
/// d=395.4545, field 0.5° full (field_diam_mm derived from F*tan(0.25°)*2
/// so theta reproduces exactly 0.25° half-field per the KB doc).
const D: f64 = 200.0;
const F: f64 = 1600.0;
const R1: f64 = 1200.0;
const R2: f64 = 654.5455;
const SEP: f64 = 395.4545;
const LAMBDA_NM: f64 = 550.0;

/// `field_diam_mm` such that `theta = atan((field_diam_mm/2)/F)` comes out
/// to exactly 0.25° half-field (0.5° full field), matching the KB doc's
/// worked example inputs precisely (rather than approximating with a
/// small-angle `2*F*tan(0.25deg)` shortcut that could drift under
/// rounding).
fn field_diam_for_half_field_deg(half_field_deg: f64, f_mm: f64) -> f64 {
    2.0 * f_mm * half_field_deg.to_radians().tan()
}

fn rc_spec() -> SeidelSpec {
    compute(
        D,
        F,
        R1,
        R2,
        -1.1454741,
        -6.5878621,
        SEP,
        field_diam_for_half_field_deg(0.25, F),
        LAMBDA_NM,
    )
}

fn classical_spec() -> SeidelSpec {
    compute(
        D,
        F,
        R1,
        R2,
        -1.0,
        -4.84,
        SEP,
        field_diam_for_half_field_deg(0.25, F),
        LAMBDA_NM,
    )
}

/// Mandatory self-check (1/3): RC's S_II is machine-zero-ish (aplanatic
/// coma null), well within the 1e-6..1e-4 mm tolerance the KB doc allows
/// for rounded (7-decimal) stored conic constants.
#[test]
fn rc_coma_s2_is_near_zero() {
    let spec = rc_spec();
    assert!(
        spec.coma_s2.abs() < 1e-6,
        "RC S_II should be ~0 (aplanatic), got {}",
        spec.coma_s2
    );
}

/// Mandatory self-check (2/3): classical Cassegrain's S_II is clearly
/// non-zero and matches the KB doc's worked value -8.5221e-4 mm.
#[test]
fn classical_coma_s2_matches_worked_example() {
    let spec = classical_spec();
    assert!(
        (spec.coma_s2 - (-8.5221e-4)).abs() < 1e-7,
        "classical S_II should be ~ -8.5221e-4 mm, got {}",
        spec.coma_s2
    );
}

/// RC astigmatism S_III matches the KB doc's worked value 3.0591e-4 mm.
#[test]
fn rc_astigmatism_s3_matches_worked_example() {
    let spec = rc_spec();
    assert!(
        (spec.astigmatism_s3 - 3.0591e-4).abs() < 1e-7,
        "got {}",
        spec.astigmatism_s3
    );
}

/// Classical astigmatism S_III matches the KB doc's worked value
/// 2.6277e-4 mm.
#[test]
fn classical_astigmatism_s3_matches_worked_example() {
    let spec = classical_spec();
    assert!(
        (spec.astigmatism_s3 - 2.6277e-4).abs() < 1e-7,
        "got {}",
        spec.astigmatism_s3
    );
}

/// Both designs share the same S_IV (Petzval sum) — same mirror curvatures
/// in both, per the KB doc's worked table (2.6442e-4 mm both).
#[test]
fn field_curvature_s4_matches_and_is_conic_independent() {
    let rc = rc_spec();
    let classical = classical_spec();
    assert!(
        (rc.field_curvature_s4 - 2.6442e-4).abs() < 1e-7,
        "RC S_IV got {}",
        rc.field_curvature_s4
    );
    assert!(
        (classical.field_curvature_s4 - 2.6442e-4).abs() < 1e-7,
        "classical S_IV got {}",
        classical.field_curvature_s4
    );
}

/// Mandatory self-check (3/3, part a): RC edge blur ≈ 0.63 arcsec
/// (astigmatism-dominated).
#[test]
fn rc_edge_blur_matches_worked_example() {
    let spec = rc_spec();
    assert!(
        (spec.edge_blur_arcsec - 0.63).abs() < 0.02,
        "got {}",
        spec.edge_blur_arcsec
    );
    assert_eq!(spec.conic_mode.as_deref(), Some("aplanatic (coma-nulled)"));
}

/// Classical edge blur ≈ 2.637 arcsec (coma-dominated).
#[test]
fn classical_edge_blur_matches_worked_example() {
    let spec = classical_spec();
    assert!(
        (spec.edge_blur_arcsec - 2.637).abs() < 0.02,
        "got {}",
        spec.edge_blur_arcsec
    );
    assert_eq!(spec.conic_mode.as_deref(), Some("coma-dominated"));
}

/// Mandatory self-check (3/3, part b) — the core headline physics
/// validation: RC's blur is SMALLER than classical's at the same field
/// (RC is aplanatic, classical is not).
#[test]
fn rc_blur_is_smaller_than_classical_blur() {
    let rc = rc_spec();
    let classical = classical_spec();
    assert!(
        rc.edge_blur_arcsec < classical.edge_blur_arcsec,
        "RC blur {} should be < classical blur {}",
        rc.edge_blur_arcsec,
        classical.edge_blur_arcsec
    );
}

/// `field_of_view_deg` reports the full field (0.5°), not the half-field
/// theta used internally for the chief-ray trace.
#[test]
fn field_of_view_deg_is_full_field_not_half() {
    let spec = rc_spec();
    assert!(
        (spec.field_of_view_deg - 0.5).abs() < 1e-6,
        "got {}",
        spec.field_of_view_deg
    );
}

/// `provisional` is `false` (S2 is VERIFIED) and `source` matches the KB
/// doc's locked citation string verbatim.
#[test]
fn provisional_false_and_source_matches_kb_doc() {
    let spec = rc_spec();
    assert!(!spec.provisional);
    assert_eq!(spec.source, "Welford ch.7 §7.2 + ch.8");
}
