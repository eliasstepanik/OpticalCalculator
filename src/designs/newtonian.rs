//! Newtonian mirror formulas: secondary sizing, obstruction, conic (−1.0).
//!
//! Real physics per formula-validation.md A1 (Berry & Kriege, *The
//! Dobsonian Telescope*; Sam Brown, *All About Telescopes*) — standard ATM
//! similar-triangles secondary-mirror sizing:
//!
//! ```text
//! minor_axis_mm   m = i + L·(D − i)/F
//! offset_mm       Δ = m/(4N)     (toward primary AND away from focuser, each)
//! obstruction     linear = m/D,  areal = (m/D)²
//! ```
//!
//! `L` (`Request::focuser_backfocus_mm`) is the FULL secondary-center-to-
//! focal-plane distance (see `request.rs` doc comment) — not drawtube-only
//! travel. `i` (`Request::field_diam_mm`) is the fully-illuminated field
//! diameter, defaulting to 20.0 mm per the documented ATM convention
//! (`request.rs`) when the user omits it.
//!
//! formula-validation.md F7: `L < D/2` is physically impossible (the light
//! path alone needs more clearance than the primary's radius to exit the
//! tube) — this pipeline rejects it via `OptError::InvalidValue` (a mirror-
//! geometry problem, not a chromatic-solver problem — `Unsolvable` is
//! reserved for the refractor chromatic solver's singular-matrix case)
//! rather than silently emitting a nonsensical secondary size.

use crate::common;
use crate::config::DesignDef;
use crate::designs::DesignPipeline;
use crate::error::OptError;
use crate::glass::GlassCatalog;
use crate::output::{DiffractionSpec, FovSpec, NewtonianSecondarySpec, SpecSheet};
use crate::request::Request;

/// ATM default fully-illuminated field diameter when the user omits
/// `field_diam_mm` (documented in `request.rs`).
const DEFAULT_FIELD_DIAM_MM: f64 = 20.0;

/// ATM default design wavelength (visual green) when the user omits
/// `wavelength_nm` (documented in `request.rs`).
const DEFAULT_WAVELENGTH_NM: f64 = 550.0;

/// Newtonian pipeline (`DesignDef.pipeline == "newtonian"`).
pub struct Newtonian;

impl DesignPipeline for Newtonian {
    fn compute(&self, req: &Request, def: &DesignDef, _glasses: &GlassCatalog) -> Result<SpecSheet, OptError> {
        let aperture_mm = req.aperture_mm;
        let f_ratio = req.f_ratio;
        let focal_length_mm = common::focal_length_mm(aperture_mm, f_ratio);

        let wavelength_nm = req.wavelength_nm.unwrap_or(DEFAULT_WAVELENGTH_NM);
        let (mag_min, mag_max) = common::useful_magnification_range(aperture_mm);
        let diffraction = Some(DiffractionSpec {
            dawes_limit_arcsec: common::dawes_limit_arcsec(aperture_mm),
            airy_disk_diameter_um: common::airy_disk_diameter_um(f_ratio, wavelength_nm),
            useful_mag_min: mag_min,
            useful_mag_max: mag_max,
        });

        let fov = if req.eyepiece_fieldstop_mm.is_some() || req.sensor_diag_mm.is_some() {
            Some(FovSpec {
                plate_scale_arcsec_per_mm: common::plate_scale_arcsec_per_mm(focal_length_mm),
                true_fov_arcmin: req
                    .eyepiece_fieldstop_mm
                    .map(|fs| common::true_fov_arcmin(fs, focal_length_mm)),
                sensor_fov_arcmin: req
                    .sensor_diag_mm
                    .map(|sd| common::sensor_fov_arcmin(sd, focal_length_mm)),
            })
        } else {
            None
        };

        let primary_conic = def.conic.as_ref().map(|c| c.primary).unwrap_or(-1.0);
        let newtonian_secondary = Some(compute_secondary(
            req,
            aperture_mm,
            f_ratio,
            focal_length_mm,
            primary_conic,
        )?);

        Ok(SpecSheet {
            design_name: req.design.clone(),
            aperture_mm,
            focal_length_mm,
            f_ratio,
            diffraction,
            fov,
            newtonian_secondary,
            cassegrain: None,
            refractor_prescription: None,
            notes: vec![
                "Predesign only — thin-mirror/paraxial similar-triangles approximation, \
                 not a fabrication-ready prescription (architecture.md §8 Q4)."
                    .to_string(),
            ],
        })
    }
}

/// Secondary (diagonal) sizing per formula-validation.md A1. `L` must be
/// present (`Request::validate` enforces this via `designs.toml`'s
/// `required` list before `compute` runs); still handled as a
/// `MissingField` rather than unwrapped, since this module has no other way
/// to guarantee the caller validated first.
fn compute_secondary(
    req: &Request,
    aperture_mm: f64,
    f_ratio: f64,
    focal_length_mm: f64,
    primary_conic: f64,
) -> Result<NewtonianSecondarySpec, OptError> {
    let l = req.focuser_backfocus_mm.ok_or_else(|| OptError::MissingField {
        design: req.design.clone(),
        field: "focuser_backfocus_mm".to_string(),
    })?;
    let i = req.field_diam_mm.unwrap_or(DEFAULT_FIELD_DIAM_MM);

    // formula-validation.md F7: L < D/2 is physically impossible — the light
    // path alone needs more clearance than the primary's radius to exit the
    // tube (request.rs documents this on `focuser_backfocus_mm`). This is a
    // mirror-geometry problem, not a chromatic-solver problem, so it uses
    // `InvalidValue` rather than `Unsolvable` (which would misleadingly talk
    // about "chromatic system" / "lower-dispersion element").
    if l < aperture_mm / 2.0 {
        let half = aperture_mm / 2.0;
        return Err(OptError::InvalidValue {
            field: "focuser_backfocus_mm".to_string(),
            reason: format!(
                "focuser_backfocus_mm ({l:.2}) is less than aperture_mm/2 ({half:.2}) \
                 — physically impossible secondary geometry; L must be the FULL \
                 secondary-center-to-focal-plane distance, not drawtube-only travel"
            ),
        });
    }

    let minor_axis_mm = i + l * (aperture_mm - i) / focal_length_mm;
    let offset_mm = minor_axis_mm / (4.0 * f_ratio);
    let obstruction_linear_pct = (minor_axis_mm / aperture_mm) * 100.0;
    let obstruction_areal_pct = (minor_axis_mm / aperture_mm).powi(2) * 100.0;

    Ok(NewtonianSecondarySpec {
        minor_axis_mm,
        offset_mm,
        obstruction_linear_pct,
        obstruction_areal_pct,
        primary_conic,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConicDef;

    fn newtonian_def() -> DesignDef {
        DesignDef {
            pipeline: "newtonian".to_string(),
            elements: 2,
            kind: "reflector".to_string(),
            required: vec![
                "aperture_mm".to_string(),
                "f_ratio".to_string(),
                "focuser_backfocus_mm".to_string(),
                "field_diam_mm".to_string(),
            ],
            optional: vec![],
            outputs: vec!["basics".to_string()],
            conic: Some(ConicDef { primary: -1.0 }),
            conic_mode: None,
            glasses: None,
            power_split: None,
            n_mirrors: None,
            fab_applicable: None,
        }
    }

    fn newtonian_request(focuser_backfocus_mm: Option<f64>, field_diam_mm: Option<f64>) -> Request {
        Request {
            design: "newtonian".to_string(),
            aperture_mm: 200.0,
            f_ratio: 6.0,
            focuser_backfocus_mm,
            field_diam_mm,
            primary_f_ratio: None,
            backfocus_mm: None,
            eyepiece_fieldstop_mm: None,
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

    /// formula-validation.md A1 worked example: D=200, f/6 (F=1200), L=220,
    /// i=20 -> m=53.00mm, Δ=2.2083mm, linear obstruction=26.5%.
    #[test]
    fn worked_example_matches_formula_validation() {
        let req = newtonian_request(Some(220.0), Some(20.0));
        let def = newtonian_def();
        let catalog = GlassCatalog::from_glasses(vec![]);

        let sheet = Newtonian.compute(&req, &def, &catalog).expect("must compute");

        assert!((sheet.focal_length_mm - 1200.0).abs() < 1e-9);

        let sec = sheet.newtonian_secondary.expect("secondary must be populated");
        assert_eq!(sec.primary_conic, -1.0);
        assert!((sec.minor_axis_mm - 53.00).abs() < 0.01, "m = {}", sec.minor_axis_mm);
        assert!((sec.offset_mm - 2.2083).abs() < 0.001, "offset = {}", sec.offset_mm);
        assert!(
            (sec.obstruction_linear_pct - 26.5).abs() < 0.1,
            "linear = {}",
            sec.obstruction_linear_pct
        );
        assert!(
            (sec.obstruction_areal_pct - 7.0225).abs() < 0.01,
            "areal = {}",
            sec.obstruction_areal_pct
        );

        assert!(sheet.diffraction.is_some());
        assert!(sheet.fov.is_none()); // no eyepiece/sensor field given
        assert_eq!(sheet.notes.len(), 1);
    }

    /// formula-validation.md F7: L < D/2 must error, not silently compute a
    /// nonsensical secondary size.
    #[test]
    fn l_less_than_half_aperture_errs() {
        let req = newtonian_request(Some(50.0), Some(20.0)); // D=200, L=50 < 100
        let def = newtonian_def();
        let catalog = GlassCatalog::from_glasses(vec![]);

        let err = Newtonian.compute(&req, &def, &catalog).expect_err("must reject L < D/2");
        match err {
            OptError::InvalidValue { field, reason } => {
                assert_eq!(field, "focuser_backfocus_mm");
                assert!(reason.contains("physically impossible"), "reason = {reason}");
                assert!(
                    !reason.to_lowercase().contains("chromatic") && !reason.to_lowercase().contains("dispersion"),
                    "reason must not mention chromatic solver terms, got: {reason}"
                );
            }
            other => panic!("expected InvalidValue, got {other:?}"),
        }
    }

    /// Missing `focuser_backfocus_mm` (not caught upstream, e.g. a caller
    /// that skips `Request::validate`) errs cleanly rather than panicking.
    #[test]
    fn missing_backfocus_errs_as_missing_field() {
        let req = newtonian_request(None, Some(20.0));
        let def = newtonian_def();
        let catalog = GlassCatalog::from_glasses(vec![]);

        let err = Newtonian.compute(&req, &def, &catalog).expect_err("must fail without L");
        match err {
            OptError::MissingField { field, .. } => assert_eq!(field, "focuser_backfocus_mm"),
            other => panic!("expected MissingField, got {other:?}"),
        }
    }

    /// FOV section only populates when eyepiece/sensor inputs are given.
    #[test]
    fn fov_populates_when_eyepiece_fieldstop_given() {
        let mut req = newtonian_request(Some(220.0), Some(20.0));
        req.eyepiece_fieldstop_mm = Some(27.5);
        let def = newtonian_def();
        let catalog = GlassCatalog::from_glasses(vec![]);

        let sheet = Newtonian.compute(&req, &def, &catalog).expect("must compute");
        let fov = sheet.fov.expect("fov must be populated");
        assert!(fov.true_fov_arcmin.is_some());
        assert!(fov.sensor_fov_arcmin.is_none());
    }
}
