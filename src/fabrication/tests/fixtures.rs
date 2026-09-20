//! Shared test fixtures for `fabrication::tests::{phase_a, phase_b}`.

use crate::coating::{Coating, CoatingCatalog};
use crate::config::DesignDef;
use crate::output::{CassegrainSpec, SpecSheet};
use crate::request::Request;
use crate::substrate::{Substrate, SubstrateCatalog};

pub(super) fn pyrex_catalog() -> SubstrateCatalog {
    SubstrateCatalog::from_substrates(vec![Substrate {
        name: "pyrex".to_string(),
        cte_ppm_per_k: 3.25,
        density_g_cm3: 2.23,
        cost_tier: 2,
        max_aperture_mm: 500.0,
        youngs_modulus_gpa: 63.0,
    }])
}

pub(super) fn enhanced_aluminum_catalog() -> CoatingCatalog {
    CoatingCatalog::from_coatings(vec![Coating {
        name: "enhanced_aluminum".to_string(),
        reflectivity_pct: 94.0,
    }])
}

/// Minimal Newtonian `SpecSheet` fixture — mirrors
/// `designs/newtonian.rs`'s test fixtures (basics only).
pub(super) fn newtonian_sheet() -> SpecSheet {
    newtonian_sheet_with(200.0, 1200.0, 6.0)
}

pub(super) fn newtonian_sheet_with(aperture_mm: f64, focal_length_mm: f64, f_ratio: f64) -> SpecSheet {
    SpecSheet {
        design_name: "newtonian".to_string(),
        aperture_mm,
        focal_length_mm,
        f_ratio,
        diffraction: None,
        fov: None,
        newtonian_secondary: None,
        cassegrain: None,
        refractor_prescription: None,
        notes: vec![],
    }
}

pub(super) fn newtonian_def(fab_applicable: Option<Vec<String>>) -> DesignDef {
    DesignDef {
        pipeline: "newtonian".to_string(),
        elements: 2,
        kind: "reflector".to_string(),
        required: vec!["aperture_mm".to_string(), "f_ratio".to_string()],
        optional: vec![],
        outputs: vec!["basics".to_string()],
        conic: None,
        conic_mode: None,
        glasses: None,
        power_split: None,
        n_mirrors: Some(2),
        fab_applicable,
    }
}

/// A2 RC worked-example `SpecSheet` fixture ([[formula-validation]] A2 /
/// [[formula-validation-v2]] T1/B1): D=200, F=1600, b=150, d=395.4545,
/// K1=-1.1454741, K2=-6.5878621, R1=1200, R2=654.5455.
pub(super) fn rc_sheet() -> SpecSheet {
    SpecSheet {
        design_name: "rc".to_string(),
        cassegrain: Some(CassegrainSpec {
            k1_primary_conic: -1.1454741,
            k2_secondary_conic: -6.5878621,
            back_focus_mm: 150.0,
            secondary_magnification: 8.0 / 3.0,
            mirror_separation_mm: 395.4545,
            r1_mm: 1200.0,
            r2_mm: 654.5455,
        }),
        ..newtonian_sheet_with(200.0, 1600.0, 8.0)
    }
}

pub(super) fn rc_def(fab_applicable: Option<Vec<String>>) -> DesignDef {
    DesignDef {
        pipeline: "cassegrain".to_string(),
        elements: 2,
        kind: "reflector".to_string(),
        required: vec![
            "aperture_mm".to_string(),
            "f_ratio".to_string(),
            "primary_f_ratio".to_string(),
            "backfocus_mm".to_string(),
        ],
        optional: vec![],
        outputs: vec!["basics".to_string(), "cassegrain".to_string()],
        conic: None,
        conic_mode: Some("rc".to_string()),
        glasses: None,
        power_split: None,
        n_mirrors: Some(2),
        fab_applicable,
    }
}

pub(super) fn refractor_def() -> DesignDef {
    DesignDef {
        pipeline: "refractor".to_string(),
        elements: 2,
        kind: "refractor".to_string(),
        required: vec!["aperture_mm".to_string(), "f_ratio".to_string()],
        optional: vec![],
        outputs: vec!["basics".to_string(), "prescription".to_string()],
        conic: None,
        conic_mode: None,
        glasses: Some(vec!["BK7".to_string()]),
        power_split: None,
        n_mirrors: None,
        fab_applicable: None,
    }
}

pub(super) fn dummy_request() -> Request {
    Request {
        design: "newtonian".to_string(),
        aperture_mm: 200.0,
        f_ratio: 6.0,
        focuser_backfocus_mm: None,
        field_diam_mm: None,
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
