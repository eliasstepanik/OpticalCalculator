//! Surface-figure tolerance physics (#4, λ/8 part ONLY,
//! [[v2-tolerance-collimation]] `## #4 — Fabrication tolerance budget (λ/4
//! Rayleigh) — STANDARD`, `### Surface-figure tolerance`) — Chunk V7,
//! [[task-snapshot-v2-phaseA]] ("V7 | fabrication/tolerance.rs (#4 λ/8
//! ONLY) ... λ/8 surface P-V (VERIFIED); ΔK deferred to T1/VB2;
//! ToleranceSpec subset").
//!
//! Reference: Rayleigh quarter-wave criterion (Born & Wolf, *Principles of
//! Optics*, §9.3; Schroeder, *Astronomical Optics*, ch. 10; Malacara,
//! *Optical Shop Testing*) — the standard optics-shop rule that a system is
//! "diffraction limited" when the peak-to-valley wavefront error does not
//! exceed λ/4. A reflecting surface's error is double-passed by the
//! wavefront (the light reflects off the same deviation twice, once on the
//! way in as far as the wavefront's phase delay is concerned), so the
//! *surface* P-V figure tolerance for a mirror is λ/8 — the well-known
//! "λ/8 rule" cited throughout amateur telescope making (ATM) literature
//! (e.g. Suiter, *Star Testing Astronomical Telescopes*).
//!
//! **Chunk V7 scope — Phase A ONLY:** this file implements *only* the λ/8
//! surface-figure / λ/4 wavefront Rayleigh criterion, which
//! [[formula-validation-v2]] records as VERIFIED ("T1 | #4 conic-tol
//! constant | PROVISIONAL (λ/8 part VERIFIED)"). It deliberately does
//! **NOT** implement:
//!
//! - The conic-constant tolerance formula
//!   (`ΔW ≈ (1/8)·ΔK·y⁴/R³` / `ΔK_tol = 2·λ·R³/y⁴`) — this is item **T1**,
//!   Phase B-gated, pending its own literature-validation pass
//!   ([[formula-validation-v2]] "Confirm the numeric constant ... against
//!   Schroeder ch.10 / Malacara aspheric-sag ... before build").
//! - Any collimation / tilt-coma coefficient (#5, item **C1**) — Phase C,
//!   BLOCKED pending a dedicated `@ultrathink-high` validation pass.
//!
//! `ToleranceSpec` here is therefore the Phase-A **subset** of
//! [[v2-tolerance-collimation]]'s full (Phase A+B) struct sketch: it
//! carries only the wavelength and the surface/wavefront λ/8 and λ/4
//! fields. The doc's `primary_conic_tol`/`secondary_conic_tol`/
//! `secondary_conic_tol_fractional`/`conic_mode` fields are Phase-B-only
//! and are not present on this struct at all (rather than being filled
//! with a guessed or placeholder numeric value) — adding them is Chunk
//! VB2's job once T1 is signed off in [[formula-validation-v2]].
//!
//! Wiring this module into `fabrication::compute`'s dispatch match
//! (`fabrication/mod.rs`, Chunk V2) and adding a `tolerance` field to
//! `FabricationReport` are Chunk V9's job, per
//! [[task-snapshot-v2-phaseA]]'s chunk table — deliberately left untouched
//! here to stay scoped to this one new file.

/// Phase-A surface-figure tolerance result: the Rayleigh λ/8 surface P-V
/// figure tolerance and its λ/4 reflected-wavefront counterpart, for a
/// mirror surface at `wavelength_nm`.
///
/// Subset of [[v2-tolerance-collimation]]'s `ToleranceSpec` sketch — the
/// conic-tolerance fields (`primary_conic_tol` etc., Phase B/T1) are
/// intentionally absent from this struct, not defaulted or guessed.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ToleranceSpec {
    pub wavelength_nm: f64,
    /// Rayleigh λ/8 surface peak-to-valley figure tolerance, in nanometers.
    pub surface_pv_tol_nm: f64,
    /// The same tolerance expressed in waves of `wavelength_nm` — always
    /// `0.125` (λ/8), reported alongside the nm figure per
    /// [[v2-tolerance-collimation]] ("Reported both in nm and in waves").
    pub surface_pv_tol_waves: f64,
    /// Rayleigh λ/4 reflected-wavefront peak-to-valley tolerance, in
    /// nanometers — the double-pass counterpart of `surface_pv_tol_nm`,
    /// reported for reference/display.
    pub wavefront_pv_tol_nm: f64,
    /// The wavefront tolerance in waves — always `0.25` (λ/4).
    pub wavefront_pv_tol_waves: f64,
}

/// Rayleigh λ/8 surface peak-to-valley figure tolerance, in micrometers,
/// for a mirror at `wavelength_nm`.
///
/// Formula ([[v2-tolerance-collimation]] `### Surface-figure tolerance`):
/// `surface_pv_tol_um = (wavelength_nm / 8.0) / 1000.0` (nm → µm).
pub fn surface_figure_tolerance_um(wavelength_nm: f64) -> f64 {
    (wavelength_nm / 8.0) / 1000.0
}

/// Rayleigh λ/4 reflected-wavefront peak-to-valley tolerance, in
/// micrometers, for reference/display alongside the surface tolerance.
///
/// Formula: `wavefront_pv_tol_um = (wavelength_nm / 4.0) / 1000.0`.
pub fn wavefront_error_pv_um(wavelength_nm: f64) -> f64 {
    (wavelength_nm / 4.0) / 1000.0
}

/// Compute the Phase-A `ToleranceSpec` (λ/8 surface + λ/4 wavefront) for
/// `wavelength_nm`. Infallible: `wavelength_nm` is range-validated
/// upstream (`request_validate.rs`) before a `Request` reaches this
/// module; this function performs only the two Rayleigh-criterion
/// divisions, which are well-defined for any positive `wavelength_nm`.
pub fn compute_phase_a(wavelength_nm: f64) -> ToleranceSpec {
    ToleranceSpec {
        wavelength_nm,
        surface_pv_tol_nm: wavelength_nm / 8.0,
        surface_pv_tol_waves: 0.125,
        wavefront_pv_tol_nm: wavelength_nm / 4.0,
        wavefront_pv_tol_waves: 0.25,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Worked example ([[v2-tolerance-collimation]] /
    /// [[task-snapshot-v2-phaseA]] "V7 | ... λ=550 → surface_pv_tol=68.75nm,
    /// waves=0.125"): visual default 550nm.
    /// surface_pv_tol_nm = 550/8 = 68.75; wavefront_pv_tol_nm = 550/4 =
    /// 137.5. In micrometers: 0.06875 and 0.1375 respectively.
    #[test]
    fn worked_example_550nm_surface_and_wavefront_tolerance() {
        let spec = compute_phase_a(550.0);

        assert!((spec.wavelength_nm - 550.0).abs() < 1e-12);
        assert!((spec.surface_pv_tol_nm - 68.75).abs() < 1e-12);
        assert!((spec.surface_pv_tol_waves - 0.125).abs() < 1e-12);
        assert!((spec.wavefront_pv_tol_nm - 137.5).abs() < 1e-12);
        assert!((spec.wavefront_pv_tol_waves - 0.25).abs() < 1e-12);

        // Micrometer-unit helper functions (same physics, µm units).
        assert!((surface_figure_tolerance_um(550.0) - 0.06875).abs() < 1e-12);
        assert!((wavefront_error_pv_um(550.0) - 0.1375).abs() < 1e-12);
    }

    /// Second wavelength (H-alpha, 656nm) confirms the formula generalizes
    /// rather than being hardcoded to the 550nm default.
    /// surface_pv_tol_nm = 656/8 = 82.0; wavefront_pv_tol_nm = 656/4 = 164.0.
    /// In micrometers: 0.082 and 0.164 respectively.
    #[test]
    fn h_alpha_656nm_surface_and_wavefront_tolerance() {
        let spec = compute_phase_a(656.0);

        assert!((spec.surface_pv_tol_nm - 82.0).abs() < 1e-12);
        assert!((spec.wavefront_pv_tol_nm - 164.0).abs() < 1e-12);
        assert!((spec.surface_pv_tol_waves - 0.125).abs() < 1e-12);
        assert!((spec.wavefront_pv_tol_waves - 0.25).abs() < 1e-12);

        assert!((surface_figure_tolerance_um(656.0) - 0.082).abs() < 1e-12);
        assert!((wavefront_error_pv_um(656.0) - 0.164).abs() < 1e-12);
    }

    /// The wavefront tolerance is always exactly 2x the surface tolerance
    /// (double-pass reflection relationship) regardless of wavelength —
    /// a general-shape check, not just the two worked-example points.
    #[test]
    fn wavefront_tolerance_is_always_double_surface_tolerance() {
        for wavelength_nm in [400.0, 550.0, 656.0, 1000.0] {
            let spec = compute_phase_a(wavelength_nm);
            assert!((spec.wavefront_pv_tol_nm - 2.0 * spec.surface_pv_tol_nm).abs() < 1e-9);
            assert!(
                (wavefront_error_pv_um(wavelength_nm) - 2.0 * surface_figure_tolerance_um(wavelength_nm))
                    .abs()
                    < 1e-12
            );
        }
    }
}
