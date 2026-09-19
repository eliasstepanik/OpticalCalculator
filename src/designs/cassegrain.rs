//! RC + classical Cassegrain formulas: conics, back-focus, magnification,
//! mirror separation and radii.
//!
//! Reference: Schroeder, *Astronomical Optics* 2nd ed., §6.2 (two-mirror
//! aplanat), cross-checked against published HST conics
//! (formula-validation.md A2). Symbols (all lengths mm, primary-vertex
//! convention):
//!
//! ```text
//! f1  = primary focal length   = aperture_mm * primary_f_ratio
//! F   = system focal length    = aperture_mm * f_ratio
//! m   = F / f1                 (secondary magnification)
//! b   = backfocus_mm           (primary vertex -> focal plane)
//! beta = b / f1
//! d   = f1*(m - beta)/(m + 1)         (mirror separation)
//! R1  = 2*f1                          (primary, concave)
//! R2  = 2*m*f1*(1 + beta)/(m*m - 1)   (secondary, convex)
//!
//! classical:  K1 = -1
//!             K2 = -((m+1)/(m-1))^2
//! rc:         K1 = -1 - 2*(1+beta) / (m*m*(m-beta))
//!             K2 = -((m+1)/(m-1))^2 - 2*m*(m+1) / ((m-beta)*(m-1)^3)
//! ```

use crate::common;
use crate::config::DesignDef;
use crate::designs::DesignPipeline;
use crate::error::OptError;
use crate::glass::GlassCatalog;
use crate::output::{CassegrainSpec, DiffractionSpec, FovSpec, SpecSheet};
use crate::request::Request;

/// RC + classical Cassegrain pipeline (`DesignDef.pipeline == "cassegrain"`,
/// variant selected by `DesignDef.conic_mode`).
pub struct Cassegrain;

/// Cassegrain conic + geometry result (Schroeder §6.2).
struct CassegrainGeometry {
    d_mm: f64,
    r1_mm: f64,
    r2_mm: f64,
    k1: f64,
    k2: f64,
    m: f64,
}

/// Compute mirror separation, radii, and conics for a given secondary
/// magnification `m`, backfocus fraction `beta` (of `f1`), primary focal
/// length `f1_mm`, and `conic_mode` ("rc" | "classical").
fn compute_geometry(f1_mm: f64, m: f64, beta: f64, conic_mode: &str) -> CassegrainGeometry {
    let d_mm = f1_mm * (m - beta) / (m + 1.0);
    let r1_mm = 2.0 * f1_mm;
    let r2_mm = 2.0 * m * f1_mm * (1.0 + beta) / (m * m - 1.0);

    let k1_classical = -1.0;
    let k2_classical = -((m + 1.0) / (m - 1.0)).powi(2);

    let (k1, k2) = if conic_mode == "rc" {
        let k1_rc = -1.0 - 2.0 * (1.0 + beta) / (m * m * (m - beta));
        let k2_rc = k2_classical - 2.0 * m * (m + 1.0) / ((m - beta) * (m - 1.0).powi(3));
        (k1_rc, k2_rc)
    } else {
        (k1_classical, k2_classical)
    };

    CassegrainGeometry {
        d_mm,
        r1_mm,
        r2_mm,
        k1,
        k2,
        m,
    }
}

impl DesignPipeline for Cassegrain {
    fn compute(&self, req: &Request, def: &DesignDef, _glasses: &GlassCatalog) -> Result<SpecSheet, OptError> {
        let primary_f_ratio = req.primary_f_ratio.ok_or_else(|| OptError::MissingField {
            design: req.design.clone(),
            field: "primary_f_ratio".to_string(),
        })?;
        let backfocus_mm = req.backfocus_mm.ok_or_else(|| OptError::MissingField {
            design: req.design.clone(),
            field: "backfocus_mm".to_string(),
        })?;

        let f1_mm = req.aperture_mm * primary_f_ratio;
        let focal_length_mm = common::focal_length_mm(req.aperture_mm, req.f_ratio);
        let m = req.f_ratio / primary_f_ratio;
        let beta = backfocus_mm / f1_mm;

        let conic_mode = def.conic_mode.as_deref().unwrap_or("classical");
        let geom = compute_geometry(f1_mm, m, beta, conic_mode);

        let wavelength_nm = req.wavelength_nm.unwrap_or(550.0);
        let (useful_mag_min, useful_mag_max) = common::useful_magnification_range(req.aperture_mm);
        let diffraction = Some(DiffractionSpec {
            dawes_limit_arcsec: common::dawes_limit_arcsec(req.aperture_mm),
            airy_disk_diameter_um: common::airy_disk_diameter_um(req.f_ratio, wavelength_nm),
            useful_mag_min,
            useful_mag_max,
        });

        let plate_scale = common::plate_scale_arcsec_per_mm(focal_length_mm);
        let true_fov_arcmin = req
            .eyepiece_fieldstop_mm
            .map(|stop| common::true_fov_arcmin(stop, focal_length_mm));
        let sensor_fov_arcmin = req
            .sensor_diag_mm
            .map(|diag| common::sensor_fov_arcmin(diag, focal_length_mm));
        let fov = Some(FovSpec {
            plate_scale_arcsec_per_mm: plate_scale,
            true_fov_arcmin,
            sensor_fov_arcmin,
        });

        let cassegrain = Some(CassegrainSpec {
            k1_primary_conic: geom.k1,
            k2_secondary_conic: geom.k2,
            back_focus_mm: backfocus_mm,
            secondary_magnification: geom.m,
            mirror_separation_mm: geom.d_mm,
            r1_mm: geom.r1_mm,
            r2_mm: geom.r2_mm,
        });

        Ok(SpecSheet {
            design_name: req.design.clone(),
            aperture_mm: req.aperture_mm,
            focal_length_mm,
            f_ratio: req.f_ratio,
            diffraction,
            fov,
            newtonian_secondary: None,
            cassegrain,
            refractor_prescription: None,
            notes: vec![
                "predesign only, not fabrication-ready — thin-mirror conics/back-focus, \
                 no thickness/tolerance analysis (architecture.md §8 Q4)"
                    .to_string(),
            ],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rc_request() -> Request {
        Request {
            design: "rc".to_string(),
            aperture_mm: 200.0,
            f_ratio: 8.0,
            focuser_backfocus_mm: None,
            field_diam_mm: None,
            primary_f_ratio: Some(3.0),
            backfocus_mm: Some(150.0),
            eyepiece_fieldstop_mm: None,
            sensor_diag_mm: None,
            wavelength_nm: None,
            output_path: None,
        }
    }

    fn def_with_conic_mode(conic_mode: &str) -> DesignDef {
        toml::from_str(&format!(
            r#"
                pipeline = "cassegrain"
                elements = 2
                kind = "reflector"
                required = ["aperture_mm", "f_ratio", "primary_f_ratio", "backfocus_mm"]
                outputs = ["basics", "diffraction", "plate_scale", "fov", "conic"]
                conic_mode = "{conic_mode}"
            "#
        ))
        .expect("valid inline DesignDef")
    }

    /// Worked example (formula-validation.md A2): D=200, primary f/3
    /// (f1=600), system f/8 (F=1600), b=150 -> m=8/3, beta=1/4.
    #[test]
    fn rc_worked_example_matches_formula_validation() {
        let req = rc_request();
        let def = def_with_conic_mode("rc");
        let sheet = Cassegrain.compute(&req, &def, &GlassCatalog::default()).expect("must compute");
        let c = sheet.cassegrain.expect("cassegrain section populated");

        assert!((c.k1_primary_conic - (-1.1454741)).abs() < 1e-6, "K1 = {}", c.k1_primary_conic);
        assert!((c.k2_secondary_conic - (-6.5878621)).abs() < 1e-6, "K2 = {}", c.k2_secondary_conic);
        assert!((c.mirror_separation_mm - 395.4545).abs() < 1e-3, "d = {}", c.mirror_separation_mm);
        assert!((c.r1_mm - 1200.0).abs() < 1e-3, "R1 = {}", c.r1_mm);
        assert!((c.r2_mm - 654.5455).abs() < 1e-3, "R2 = {}", c.r2_mm);
        assert!((c.secondary_magnification - 8.0 / 3.0).abs() < 1e-9);
        assert!((sheet.focal_length_mm - 1600.0).abs() < 1e-9);
    }

    #[test]
    fn classical_worked_example_matches_formula_validation() {
        let req = rc_request();
        let def = def_with_conic_mode("classical");
        let sheet = Cassegrain.compute(&req, &def, &GlassCatalog::default()).expect("must compute");
        let c = sheet.cassegrain.expect("cassegrain section populated");

        assert!((c.k1_primary_conic - (-1.0)).abs() < 1e-12, "K1 = {}", c.k1_primary_conic);
        assert!((c.k2_secondary_conic - (-4.84)).abs() < 1e-6, "K2 = {}", c.k2_secondary_conic);
        assert!((c.mirror_separation_mm - 395.4545).abs() < 1e-3, "d = {}", c.mirror_separation_mm);
        assert!((c.r2_mm - 654.5455).abs() < 1e-3, "R2 = {}", c.r2_mm);
    }

    #[test]
    fn missing_primary_f_ratio_errors_naming_it() {
        let mut req = rc_request();
        req.primary_f_ratio = None;
        let def = def_with_conic_mode("rc");
        let err = Cassegrain.compute(&req, &def, &GlassCatalog::default()).expect_err("must fail");
        match err {
            OptError::MissingField { field, .. } => assert_eq!(field, "primary_f_ratio"),
            other => panic!("expected MissingField, got {other:?}"),
        }
    }

    #[test]
    fn missing_backfocus_errors_naming_it() {
        let mut req = rc_request();
        req.backfocus_mm = None;
        let def = def_with_conic_mode("rc");
        let err = Cassegrain.compute(&req, &def, &GlassCatalog::default()).expect_err("must fail");
        match err {
            OptError::MissingField { field, .. } => assert_eq!(field, "backfocus_mm"),
            other => panic!("expected MissingField, got {other:?}"),
        }
    }
}
