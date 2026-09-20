//! Mirror-blank thickness/volume/mass physics (#1, [[v2-substrate-mass]]
//! `### Thickness` / `### Volume & mass`) — Chunk V4,
//! [[task-snapshot-v2-phaseA]]. Standard ATM blank rules (Berry & Kriege,
//! *The Dobsonian Telescope*; Texereau, *How to Make a Telescope*), all
//! closed form, no FEA.
//!
//! **Chunk V4 scope:** standalone module + `compute()` free function only.
//! Wiring into `fabrication::compute`'s dispatch match (`fabrication/mod.rs`,
//! Chunk V2) is Chunk V9's job per [[task-snapshot-v2-phaseA]]'s chunk
//! table ("V9 ... registered in fabrication::compute's dispatch match") —
//! deliberately left untouched here to stay scoped to this file only.
//!
//! `compute()` is infallible: `aperture_mm`, `thickness_ratio`, and
//! `lightweight_fraction` are already range-validated upstream by
//! `request_validate::validate_values` (Chunk V1, ranges [4.0, 12.0] and
//! [0.0, 0.8] respectively) before a `Request` ever reaches this module.

use crate::fabrication::SubstrateSpec;
use crate::substrate::Substrate;

/// Compute the mirror-blank `SubstrateSpec` for a solid or lightweighted
/// disk of the given `aperture_mm`, using `thickness_ratio` (D/t, default
/// 6.0 per the ATM D/6 rule) and `lightweight_fraction` (pocket-volume
/// fraction removed, [0.0, 0.8]) against `substrate`'s catalog density.
///
/// Formulas ([[v2-substrate-mass]] `### Thickness` / `### Volume & mass`):
/// - `t_mm = aperture_mm / thickness_ratio`
/// - `V_solid_cm3 = π · (aperture_mm/2)² · t_mm · 1e-3` (mm³ → cm³ via /1000)
/// - `V_blank_cm3 = V_solid_cm3 · (1 − lightweight_fraction)`
/// - `mass_kg = V_blank_cm3 · density_g_cm3 · 1e-3` (g → kg via /1000)
///
/// Infallible: callers must range-validate `thickness_ratio` (4.0–12.0) and
/// `lightweight_fraction` (0.0–0.8) beforehand (`request_validate.rs`).
pub fn compute(
    aperture_mm: f64,
    thickness_ratio: f64,
    lightweight_fraction: f64,
    substrate: &Substrate,
) -> SubstrateSpec {
    let thickness_mm = aperture_mm / thickness_ratio;
    let radius_mm = aperture_mm / 2.0;
    let solid_volume_cm3 = std::f64::consts::PI * radius_mm * radius_mm * thickness_mm * 1e-3;
    let blank_volume_cm3 = solid_volume_cm3 * (1.0 - lightweight_fraction);

    let solid_mass_kg = solid_volume_cm3 * substrate.density_g_cm3 * 1e-3;
    let mass_kg = blank_volume_cm3 * substrate.density_g_cm3 * 1e-3;

    let over_max_aperture = aperture_mm > substrate.max_aperture_mm;

    SubstrateSpec {
        material: substrate.name.clone(),
        thickness_mm,
        thickness_ratio,
        solid_volume_cm3,
        blank_volume_cm3,
        mass_kg,
        solid_mass_kg,
        lightweight_fraction,
        cte_ppm_per_k: substrate.cte_ppm_per_k,
        over_max_aperture,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Real Pyrex catalog entry ([[v2-substrate-mass]] `substrates.toml`
    /// table, density 2.23 g/cm³), matching `configs/substrates.toml`'s
    /// `pyrex` entry (Chunk V3) exactly.
    fn pyrex() -> Substrate {
        Substrate {
            name: "pyrex".to_string(),
            cte_ppm_per_k: 3.25,
            density_g_cm3: 2.23,
            cost_tier: 2,
            max_aperture_mm: 500.0,
            youngs_modulus_gpa: 63.0,
        }
    }

    /// Worked example ([[v2-substrate-mass]] `## Worked example`): D=300mm
    /// Pyrex, default D/6, no lightweighting.
    /// t = 300/6 = 50.0mm; V_solid = π·150²·50·1e-3 = 3534.2917352885 cm³
    /// (recomputed to full precision, not the doc's rounded ±0.01 figure);
    /// mass = 3534.2917352885·2.23·1e-3 = 7.8814705697 kg.
    #[test]
    fn worked_example_solid_300mm_pyrex_d6() {
        let spec = compute(300.0, 6.0, 0.0, &pyrex());

        assert!((spec.thickness_mm - 50.0).abs() < 1e-9);
        assert!((spec.solid_volume_cm3 - 3534.2917352885174).abs() < 1e-9);
        assert!((spec.blank_volume_cm3 - spec.solid_volume_cm3).abs() < 1e-9);
        assert!((spec.mass_kg - 7.881470569693393).abs() < 1e-9);
        assert!((spec.solid_mass_kg - 7.881470569693393).abs() < 1e-9);
        assert_eq!(spec.lightweight_fraction, 0.0);
        assert_eq!(spec.thickness_ratio, 6.0);
        assert_eq!(spec.material, "pyrex");
        assert!((spec.cte_ppm_per_k - 3.25).abs() < 1e-9);
        assert!(!spec.over_max_aperture);
    }

    /// Same 300mm Pyrex D/6 blank, `lightweight_fraction = 0.3`:
    /// V_blank = 3534.2917352885174 · 0.7 = 2474.004214701962 cm³;
    /// mass = 2474.004214701962 · 2.23 · 1e-3 = 5.517029398785375 kg —
    /// exactly 0.7 × the solid mass (7.881470569693393 · 0.7 =
    /// 5.5170293987853751), confirming proportional scaling.
    #[test]
    fn lightweighted_300mm_pyrex_scales_mass_by_0_7() {
        let spec = compute(300.0, 6.0, 0.3, &pyrex());

        assert!((spec.blank_volume_cm3 - 2474.004214701962).abs() < 1e-9);
        assert!((spec.mass_kg - 5.517029398785375).abs() < 1e-9);
        // Solid mass unaffected by lightweighting — still the full-disk figure.
        assert!((spec.solid_mass_kg - 7.881470569693393).abs() < 1e-9);
        // mass_kg == 0.7 * solid_mass_kg (proportional scaling assertion).
        assert!((spec.mass_kg - spec.solid_mass_kg * 0.7).abs() < 1e-9);
    }

    /// 200mm aperture, D/6 thickness ratio, solid Pyrex — second worked
    /// example from the delegation brief, recomputed to full precision:
    /// t = 200/6 = 33.333...mm; V_solid = π·100²·(200/6)·1e-3 =
    /// 1047197.5511965976 mm³·1e-3-scale... recomputed directly in cm³:
    /// π·100²·33.333333333333336·1e-3 = 1047.1975511965977 cm³;
    /// mass = 1047.1975511965977·2.23·1e-3 = 2.3352505391684133 kg.
    #[test]
    fn worked_example_200mm_aperture_d6_solid_pyrex() {
        let spec = compute(200.0, 6.0, 0.0, &pyrex());

        assert!((spec.thickness_mm - 33.333333333333336).abs() < 1e-9);
        assert!((spec.solid_volume_cm3 - 1047.1975511965977).abs() < 1e-6);
        assert!((spec.mass_kg - 2.3352505391684133).abs() < 1e-9);
    }

    /// `over_max_aperture` flags when `aperture_mm` exceeds the substrate's
    /// catalog `max_aperture_mm` (Pyrex: 500.0mm) — a note, not an error
    /// ([[v2-substrate-mass]] "Warn (note, not error)").
    #[test]
    fn over_max_aperture_flag_set_when_aperture_exceeds_catalog_max() {
        let spec = compute(600.0, 6.0, 0.0, &pyrex());
        assert!(spec.over_max_aperture);

        let spec_ok = compute(400.0, 6.0, 0.0, &pyrex());
        assert!(!spec_ok.over_max_aperture);
    }
}
