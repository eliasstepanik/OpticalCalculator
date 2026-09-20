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
    // v2 (`--fabrication`, Phase B, #6). architecture-v2.md §2d pins
    // `support_points >= 3` — a discrete-support gravity-sag formula with
    // 0 or 1 points is not a real support (0 divides-by-zero to `inf` sag;
    // 1 contradicts every named `support_scheme`, e.g. "9_point" with "1
    // points"). `full_disk` ignores `n_points` entirely in
    // `support::compute` (continuous-bed sag is reported as 0.0
    // regardless of the value), but the architecture doc states the bound
    // unconditionally with no scheme carve-out, and enforcing it
    // unconditionally costs nothing (a `full_disk` request has no reason
    // to send fewer than 3 anyway) — so this check applies to every
    // scheme alike, the simplest correct reading of §2d.
    if let Some(v) = req.support_n_points {
        if v < 3 {
            return Err(OptError::InvalidValue {
                field: "support_n_points".to_string(),
                reason: format!("must be >= 3, got {v}"),
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
mod tests;
