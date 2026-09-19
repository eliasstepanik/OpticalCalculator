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
}
