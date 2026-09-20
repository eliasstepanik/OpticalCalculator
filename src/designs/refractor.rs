//! N-element refractor pipeline: element powers → radii, prescription.
//!
//! Real physics per architecture.md §3 (N-element thin-lens chromatic
//! solver) and formula-validation.md's Chunk 13 worked example. Same
//! `RefractorPipeline` type handles doublet (N=2), triplet (N=3), and
//! quintuplet (N=5) — the only difference is `DesignDef.glasses.len()` and
//! whether `refractor_chromatic_solve::solve_powers` needs extra DOF rows
//! (N>3), which that module already handles internally.
//!
//! **Cemented-in-contact assumption (documented, not user-resolved):**
//! architecture.md §3 defines the whole N-element system as "a thin,
//! in-contact system of N elements" for the chromatic solve — this pipeline
//! carries that same model through to the radii: every element's front
//! surface `R1` is cemented to the previous element's `R2` (element 0's
//! front surface is the only free shape, chosen via the Coddington
//! minimum-spherical-aberration factor). This is the literal continuation
//! of architecture's own "in-contact" model, not an independent heuristic —
//! it is what "in contact" means once you go from powers to physical radii.
//! An air-spaced triplet/quintuplet is a distinct, more complex design
//! (needing per-gap thickness data this predesign-only tool doesn't carry)
//! and is out of scope; if a future design needs air gaps, that is a new
//! `DesignDef` field + pipeline branch, not a change to this default.
//!
//! Decision #2 (symmetric shape default, no spherical-aberration shape
//! optimization beyond the doublet's Coddington minimum): every element
//! after the first has no shape freedom anyway (its `R1` is cemented), so
//! "symmetric default" only actually applies to element 0, which already
//! uses the Coddington-minimum shape — there is no separate "equiconvex"
//! branch to add on top.

use crate::common;
use crate::config::DesignDef;
use crate::designs::refractor_shape::{cemented_r2, coddington_radii};
use crate::designs::DesignPipeline;
use crate::error::OptError;
use crate::glass::{Glass, GlassCatalog};
use crate::output::{DiffractionSpec, FovSpec, RefractorElementSpec, RefractorPrescriptionSpec, SpecSheet};
use crate::refractor_chromatic_solve::solve_powers;
use crate::request::Request;

/// ATM default design wavelength (visual green) when the user omits
/// `wavelength_nm` — same convention as `newtonian.rs`/`cassegrain.rs`.
const DEFAULT_WAVELENGTH_NM: f64 = 550.0;

/// N-element refractor pipeline (`DesignDef.pipeline == "refractor"`, N from
/// `DesignDef.elements`/`glasses.len()` — doublet/triplet/quintuplet all
/// share this type per architecture.md §3's "one solver, not three paths").
pub struct Refractor;

impl DesignPipeline for Refractor {
    fn compute(&self, req: &Request, def: &DesignDef, glasses: &GlassCatalog) -> Result<SpecSheet, OptError> {
        let glass_names = def.glasses.as_ref().ok_or_else(|| OptError::MissingField {
            design: req.design.clone(),
            field: "glasses".to_string(),
        })?;

        let resolved: Vec<&Glass> = glass_names
            .iter()
            .map(|name| glasses.get(name))
            .collect::<Result<Vec<_>, OptError>>()?;

        let focal_length_mm = common::focal_length_mm(req.aperture_mm, req.f_ratio);

        let powers = solve_powers(focal_length_mm, &resolved, def.power_split.as_deref())?;

        let elements = build_prescription(&resolved, &powers);

        let wavelength_nm = req.wavelength_nm.unwrap_or(DEFAULT_WAVELENGTH_NM);
        let (mag_min, mag_max) = common::useful_magnification_range(req.aperture_mm);
        let diffraction = Some(DiffractionSpec {
            dawes_limit_arcsec: common::dawes_limit_arcsec(req.aperture_mm),
            airy_disk_diameter_um: common::airy_disk_diameter_um(req.f_ratio, wavelength_nm),
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

        let mut notes = vec![
            "predesign only, not fabrication-ready, thin-lens approximation".to_string(),
            "element shapes default to symmetric (equiconvex/equiconcave), no \
             spherical-aberration optimization"
                .to_string(),
        ];
        if resolved.len() == 5 {
            notes.push(
                "quintuplet power-split uses a documented predesign heuristic (equal-power \
                 duplicate-glass grouping + mirror-symmetric pairing), not an optimized design."
                    .to_string(),
            );
        }

        Ok(SpecSheet {
            design_name: req.design.clone(),
            aperture_mm: req.aperture_mm,
            focal_length_mm,
            f_ratio: req.f_ratio,
            diffraction,
            fov,
            newtonian_secondary: None,
            cassegrain: None,
            refractor_prescription: Some(RefractorPrescriptionSpec { elements }),
            notes,
        })
    }
}

/// Convert element powers to a full radii prescription per the module doc's
/// cemented-in-contact-chain assumption: element 0's front surface is free
/// (Coddington minimum-SA shape); every subsequent element's front surface
/// is cemented to the previous element's rear surface, so only its rear
/// surface is solved from its own power equation (formula-validation.md
/// Chunk 13's doublet worked example is exactly this chain for N=2).
fn build_prescription(glasses: &[&Glass], powers: &[f64]) -> Vec<RefractorElementSpec> {
    let mut elements = Vec::with_capacity(glasses.len());
    let mut prev_r2: Option<f64> = None;

    for (glass, &power) in glasses.iter().zip(powers.iter()) {
        let (r1, r2) = match prev_r2 {
            None => coddington_radii(power, glass.nd),
            Some(r1_forced) => (r1_forced, cemented_r2(power, glass.nd, r1_forced)),
        };

        elements.push(RefractorElementSpec {
            glass_name: glass.name.clone(),
            power_mm_inv: power,
            r1_mm: r1,
            r2_mm: r2,
        });
        prev_r2 = Some(r2);
    }

    elements
}

#[cfg(test)]
mod tests {
    use super::*;

    fn design_def(glasses: Vec<&str>, power_split: Option<Vec<f64>>) -> DesignDef {
        DesignDef {
            pipeline: "refractor".to_string(),
            elements: glasses.len(),
            kind: "refractor".to_string(),
            required: vec!["aperture_mm".to_string(), "f_ratio".to_string()],
            optional: vec![],
            outputs: vec!["basics".to_string(), "prescription".to_string()],
            conic: None,
            conic_mode: None,
            glasses: Some(glasses.into_iter().map(str::to_string).collect()),
            power_split,
            n_mirrors: None,
            fab_applicable: None,
        }
    }

    fn request(aperture_mm: f64, f_ratio: f64) -> Request {
        Request {
            design: "test_refractor".to_string(),
            aperture_mm,
            f_ratio,
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

    fn test_catalog() -> GlassCatalog {
        GlassCatalog::from_glasses(vec![
            Glass { name: "BK7".to_string(), nd: 1.51680, vd: 64.17, pgf: 0.5349 },
            Glass { name: "F2".to_string(), nd: 1.62004, vd: 36.37, pgf: 0.5828 },
            Glass { name: "FPL53".to_string(), nd: 1.43875, vd: 94.66, pgf: 0.5340 },
        ])
    }

    /// formula-validation.md Chunk 13 doublet worked example: BK7/F2,
    /// f=1000mm, D such that f_ratio*D=1000 -> aperture 100mm, f/10.
    #[test]
    fn doublet_worked_example_matches_formula_validation() {
        let def = design_def(vec!["BK7", "F2"], None);
        let req = request(100.0, 10.0); // focal_length = 1000mm
        let catalog = test_catalog();

        let sheet = Refractor.compute(&req, &def, &catalog).expect("doublet must compute");
        assert!((sheet.focal_length_mm - 1000.0).abs() < 1e-9);

        let presc = sheet.refractor_prescription.expect("prescription populated");
        assert_eq!(presc.elements.len(), 2);

        let crown = &presc.elements[0];
        assert_eq!(crown.glass_name, "BK7");
        assert!((crown.r1_mm - 257.4).abs() < 0.5, "crown R1 = {}", crown.r1_mm);
        assert!((crown.r2_mm - (-1720.2)).abs() < 0.5, "crown R2 = {}", crown.r2_mm);

        let flint = &presc.elements[1];
        assert_eq!(flint.glass_name, "F2");
        assert!((flint.r1_mm - crown.r2_mm).abs() < 1e-9, "cemented contact surface must match");
        assert!((flint.r2_mm - 654.2).abs() < 0.5, "flint R2 = {}", flint.r2_mm);

        assert!(sheet.notes.iter().any(|n| n.contains("predesign only")));
        assert!(sheet.notes.iter().any(|n| n.contains("symmetric")));
    }

    /// Triplet end-to-end with real catalog glasses (FPL53/BK7/F2) — 3 valid
    /// finite radii sets, both required caveat notes present.
    #[test]
    fn triplet_end_to_end_produces_three_finite_radii_sets() {
        let def = design_def(vec!["FPL53", "BK7", "F2"], None);
        let req = request(100.0, 10.0);
        let catalog = test_catalog();

        let sheet = Refractor.compute(&req, &def, &catalog).expect("triplet must compute");
        let presc = sheet.refractor_prescription.expect("prescription populated");
        assert_eq!(presc.elements.len(), 3);
        for el in &presc.elements {
            assert!(el.r1_mm.is_finite(), "{} R1 not finite: {}", el.glass_name, el.r1_mm);
            assert!(el.r2_mm.is_finite(), "{} R2 not finite: {}", el.glass_name, el.r2_mm);
        }
        // Cemented chain: each element's R1 equals the previous element's R2.
        assert!((presc.elements[1].r1_mm - presc.elements[0].r2_mm).abs() < 1e-9);
        assert!((presc.elements[2].r1_mm - presc.elements[1].r2_mm).abs() < 1e-9);

        assert!(sheet.notes.iter().any(|n| n.contains("predesign only")));
        assert!(sheet.notes.iter().any(|n| n.contains("symmetric")));
        assert!(!sheet.notes.iter().any(|n| n.contains("quintuplet")));
    }

    /// Quintuplet end-to-end using the Chunk 10 duplicate-glass example
    /// [FPL53, FPL53, BK7, F2, BK7] — 5 finite radii sets, all 3 caveats.
    #[test]
    fn quintuplet_end_to_end_produces_five_finite_radii_sets_with_all_caveats() {
        let def = design_def(vec!["FPL53", "FPL53", "BK7", "F2", "BK7"], None);
        let req = request(100.0, 10.0);
        let catalog = test_catalog();

        let sheet = Refractor.compute(&req, &def, &catalog).expect("quintuplet must compute");
        let presc = sheet.refractor_prescription.expect("prescription populated");
        assert_eq!(presc.elements.len(), 5);
        for el in &presc.elements {
            assert!(el.r1_mm.is_finite(), "{} R1 not finite: {}", el.glass_name, el.r1_mm);
            assert!(el.r2_mm.is_finite(), "{} R2 not finite: {}", el.glass_name, el.r2_mm);
        }

        assert!(sheet.notes.iter().any(|n| n.contains("predesign only")));
        assert!(sheet.notes.iter().any(|n| n.contains("symmetric")));
        assert!(sheet.notes.iter().any(|n| n.contains("quintuplet power-split")));
    }

    /// Unknown glass name in `def.glasses` -> `OptError::UnknownGlass`, not
    /// a panic.
    #[test]
    fn unknown_glass_name_errors_not_panics() {
        let def = design_def(vec!["BK7", "SF99"], None);
        let req = request(100.0, 10.0);
        let catalog = test_catalog();

        let err = Refractor.compute(&req, &def, &catalog).expect_err("unknown glass must error");
        match err {
            OptError::UnknownGlass(name) => assert_eq!(name, "SF99"),
            other => panic!("expected UnknownGlass, got {other:?}"),
        }
    }
}
