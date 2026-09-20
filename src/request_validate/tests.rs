//! `#[cfg(test)] mod tests` body for `request_validate.rs`.

use super::*;

/// A fully-populated, all-valid `Request` — tests mutate one field at a
/// time off this baseline.
fn valid_request() -> Request {
    Request {
        design: "newtonian".to_string(),
        aperture_mm: 200.0,
        f_ratio: 6.0,
        focuser_backfocus_mm: Some(220.0),
        field_diam_mm: Some(20.0),
        primary_f_ratio: None,
        backfocus_mm: None,
        eyepiece_fieldstop_mm: Some(27.5),
        sensor_diag_mm: None,
        wavelength_nm: None,
        output_path: None,
        substrate: None,
        lightweight_fraction: None,
        coating: None,
        thickness_ratio: None,
        support_scheme: None,
        support_n_points: None,
    }
}

#[test]
fn valid_request_passes_value_checks() {
    assert!(validate_values(&valid_request()).is_ok());
}

#[test]
fn negative_aperture_mm_is_invalid_value_not_panic() {
    let mut req = valid_request();
    req.aperture_mm = -200.0;
    let err = validate_values(&req).expect_err("must reject negative aperture");
    match err {
        OptError::InvalidValue { field, .. } => assert_eq!(field, "aperture_mm"),
        other => panic!("expected InvalidValue, got {other:?}"),
    }
}

#[test]
fn zero_f_ratio_is_invalid_value() {
    let mut req = valid_request();
    req.f_ratio = 0.0;
    let err = validate_values(&req).expect_err("must reject zero f_ratio");
    match err {
        OptError::InvalidValue { field, .. } => assert_eq!(field, "f_ratio"),
        other => panic!("expected InvalidValue, got {other:?}"),
    }
}

#[test]
fn nan_aperture_mm_is_invalid_value_not_panic() {
    let mut req = valid_request();
    req.aperture_mm = f64::NAN;
    let err = validate_values(&req).expect_err("must reject NaN aperture");
    match err {
        OptError::InvalidValue { field, .. } => assert_eq!(field, "aperture_mm"),
        other => panic!("expected InvalidValue, got {other:?}"),
    }
}

/// Regression: `eyepiece_fieldstop_mm` was omitted from `validate_values`
/// entirely, so a negative field-stop diameter (physically impossible)
/// passed validation and produced a negative true-FOV with exit code 0.
/// See `notes/optical-calculator-request-range-validation-gap.md`.
#[test]
fn negative_eyepiece_fieldstop_mm_is_invalid_value() {
    let mut req = valid_request();
    req.eyepiece_fieldstop_mm = Some(-27.5);
    let err = validate_values(&req).expect_err("must reject negative eyepiece fieldstop");
    match err {
        OptError::InvalidValue { field, .. } => assert_eq!(field, "eyepiece_fieldstop_mm"),
        other => panic!("expected InvalidValue, got {other:?}"),
    }
}

#[test]
fn zero_eyepiece_fieldstop_mm_is_valid() {
    // Zero is a physically valid point field stop, same rule as
    // field_diam_mm/sensor_diag_mm.
    let mut req = valid_request();
    req.eyepiece_fieldstop_mm = Some(0.0);
    assert!(validate_values(&req).is_ok());
}

// v2 Chunk V1 acceptance check (3): `lightweight_fraction` range
// validation rejects negative/>0.8/NaN and accepts valid values,
// via the existing `validate_values()` pattern.

#[test]
fn negative_lightweight_fraction_is_invalid_value() {
    let mut req = valid_request();
    req.lightweight_fraction = Some(-0.1);
    let err = validate_values(&req).expect_err("must reject negative lightweight_fraction");
    match err {
        OptError::InvalidValue { field, .. } => assert_eq!(field, "lightweight_fraction"),
        other => panic!("expected InvalidValue, got {other:?}"),
    }
}

#[test]
fn over_point_eight_lightweight_fraction_is_invalid_value() {
    let mut req = valid_request();
    req.lightweight_fraction = Some(0.81);
    let err = validate_values(&req).expect_err("must reject lightweight_fraction > 0.8");
    match err {
        OptError::InvalidValue { field, .. } => assert_eq!(field, "lightweight_fraction"),
        other => panic!("expected InvalidValue, got {other:?}"),
    }
}

#[test]
fn nan_lightweight_fraction_is_invalid_value_not_panic() {
    let mut req = valid_request();
    req.lightweight_fraction = Some(f64::NAN);
    let err = validate_values(&req).expect_err("must reject NaN lightweight_fraction");
    match err {
        OptError::InvalidValue { field, .. } => assert_eq!(field, "lightweight_fraction"),
        other => panic!("expected InvalidValue, got {other:?}"),
    }
}

#[test]
fn valid_lightweight_fraction_values_pass() {
    let mut req = valid_request();
    for v in [0.0, 0.3, 0.8] {
        req.lightweight_fraction = Some(v);
        assert!(validate_values(&req).is_ok(), "expected {v} to be valid");
    }
}

#[test]
fn thickness_ratio_out_of_range_is_invalid_value() {
    let mut req = valid_request();
    req.thickness_ratio = Some(2.0);
    let err = validate_values(&req).expect_err("must reject thickness_ratio below 4.0");
    match err {
        OptError::InvalidValue { field, .. } => assert_eq!(field, "thickness_ratio"),
        other => panic!("expected InvalidValue, got {other:?}"),
    }
}

#[test]
fn valid_thickness_ratio_passes() {
    let mut req = valid_request();
    req.thickness_ratio = Some(6.0);
    assert!(validate_values(&req).is_ok());
}

/// Regression (F1, quality-gate round 1): `support_n_points=0` reached
/// `support::support_sag_nm` unvalidated and divided by `n*n == 0`,
/// producing `RMS sag: inf nm` at exit code 0 instead of a validation
/// error. architecture-v2.md §2d requires `support_points >= 3`.
#[test]
fn zero_support_n_points_is_invalid_value() {
    let mut req = valid_request();
    req.support_n_points = Some(0);
    let err = validate_values(&req).expect_err("must reject support_n_points=0");
    match err {
        OptError::InvalidValue { field, .. } => assert_eq!(field, "support_n_points"),
        other => panic!("expected InvalidValue, got {other:?}"),
    }
}

/// Regression (F1): `support_n_points=1` produced a scheme/count
/// contradiction (e.g. "9_point" scheme reported with "1 points") and
/// a plausible-looking but physically meaningless sag value.
#[test]
fn one_support_n_points_is_invalid_value() {
    let mut req = valid_request();
    req.support_n_points = Some(1);
    let err = validate_values(&req).expect_err("must reject support_n_points=1");
    match err {
        OptError::InvalidValue { field, .. } => assert_eq!(field, "support_n_points"),
        other => panic!("expected InvalidValue, got {other:?}"),
    }
}

#[test]
fn valid_support_n_points_passes() {
    let mut req = valid_request();
    req.support_n_points = Some(9);
    assert!(validate_values(&req).is_ok());
}
