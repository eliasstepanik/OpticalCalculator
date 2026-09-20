//! Physical sanity checks for `Request`'s numeric fields.
//!
//! Split out of `request.rs` (which now only owns the `Request` struct and
//! presence-only `validate`) to keep both files under the 300-line limit.
//! See `notes/optical-calculator-request-range-validation-gap.md` in the KB
//! for why presence-only validation is insufficient on its own.

use crate::error::OptError;
use crate::request::Request;

/// Physical sanity check for every numeric field that IS present.
/// Presence (`Request::validate`) says a field was supplied; this says the
/// supplied value is physically possible — negative apertures, zero
/// f-ratios, and NaN backfocus distances all pass a presence check but
/// must not reach a design pipeline and produce nonsensical output
/// (e.g. a negative Dawes limit) with exit code 0.
///
/// Returns the FIRST invalid value found as `OptError::InvalidValue`
/// (matches `validate`'s "first missing field" semantics).
pub(crate) fn validate_values(req: &Request) -> Result<(), OptError> {
    positive_finite("aperture_mm", req.aperture_mm)?;
    positive_finite("f_ratio", req.f_ratio)?;
    if let Some(v) = req.primary_f_ratio {
        positive_finite("primary_f_ratio", v)?;
    }
    if let Some(v) = req.backfocus_mm {
        positive_finite("backfocus_mm", v)?;
    }
    if let Some(v) = req.focuser_backfocus_mm {
        positive_finite("focuser_backfocus_mm", v)?;
    }
    if let Some(v) = req.field_diam_mm {
        non_negative_finite("field_diam_mm", v)?;
    }
    if let Some(v) = req.eyepiece_fieldstop_mm {
        non_negative_finite("eyepiece_fieldstop_mm", v)?;
    }
    if let Some(v) = req.sensor_diag_mm {
        non_negative_finite("sensor_diag_mm", v)?;
    }
    if let Some(v) = req.wavelength_nm {
        positive_finite("wavelength_nm", v)?;
        if !(100.0..=2000.0).contains(&v) {
            return Err(OptError::InvalidValue {
                field: "wavelength_nm".to_string(),
                reason: format!("must be in the realistic optical range 100-2000nm, got {v}"),
            });
        }
    }
    // v2 (`--fabrication`, Phase A, #1). `lightweight_fraction` is a pocket
    // volume fraction removed from a solid blank. KB architecture-v2.md
    // §2d + v2-substrate-mass.md both pin the bound at [0.0, 0.8] (not
    // 1.0 — a fully-hollowed 100% pocket-fraction blank is not physical).
    // Only checked when present, matching every other v2-guarded field.
    if let Some(v) = req.lightweight_fraction {
        if !v.is_finite() || !(0.0..=0.8).contains(&v) {
            return Err(OptError::InvalidValue {
                field: "lightweight_fraction".to_string(),
                reason: format!("must be finite and in [0.0, 0.8], got {v}"),
            });
        }
    }
    // v2 (`--fabrication`, Phase A, #1). `thickness_ratio` overrides the
    // default D/6 ATM rule — see KB doc v2-substrate-mass.md sanity bound
    // "4.0 <= thickness_ratio <= 12.0".
    if let Some(v) = req.thickness_ratio {
        if !v.is_finite() || !(4.0..=12.0).contains(&v) {
            return Err(OptError::InvalidValue {
                field: "thickness_ratio".to_string(),
                reason: format!("must be finite and in [4.0, 12.0], got {v}"),
            });
        }
    }
    Ok(())
}

/// `value` must be finite and strictly greater than zero (apertures,
/// f-ratios, backfocus distances — none of these can be zero or negative).
fn positive_finite(field: &str, value: f64) -> Result<(), OptError> {
    if !value.is_finite() || value <= 0.0 {
        return Err(OptError::InvalidValue {
            field: field.to_string(),
            reason: format!("must be positive and finite, got {value}"),
        });
    }
    Ok(())
}

/// `value` must be finite and >= zero (field/sensor diameters — a zero
/// diameter, a point field, is physically valid; negative is not).
fn non_negative_finite(field: &str, value: f64) -> Result<(), OptError> {
    if !value.is_finite() || value < 0.0 {
        return Err(OptError::InvalidValue {
            field: field.to_string(),
            reason: format!("must be non-negative and finite, got {value}"),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
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
}
