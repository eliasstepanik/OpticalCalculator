//! Collimation / alignment sensitivity physics (#5,
//! [[v2-tolerance-collimation]] `## #5 — Collimation / alignment
//! sensitivity`) — Chunk VC1, [[formula-validation-v2]] item **C1**
//! (VERIFIED 2026-09-20, 5th pass). Secondary decenter/tilt-to-coma
//! coefficients for a two-mirror (Cassegrain-family) telescope, closed
//! form, no ray tracing.
//!
//! **Reference:** Scaduto, Sasian, Stefani & Castro Neto, "Two-mirror
//! telescope design with third-order coma insensitive to decenter
//! misalignment," *Optics Express* 21(6):6851–6867 (2013), DOI
//! 10.1364/OE.21.006851, Eqs. 23–26 & 28, tracing to Wilson, *Reflecting
//! Telescope Optics I*, 2nd ed., ch. 3 pp. 61–62 & 260–267; corroborated
//! by Sacek (telescope-optics.net/two-mirror2.htm); independently verified
//! against an exact nonlinear ray trace (< 0.02 % on all coefficients,
//! see [[formula-validation-v2]] C1).
//!
//! **Locked formulas (K2-explicit form, [[v2-tolerance-collimation]]
//! "Locked formulas" section).** With `y` = aperture semi-diameter (mm),
//! `F` = system focal length (mm), `m` = secondary magnification, `R2` =
//! secondary radius of curvature magnitude (mm, v1 convention), `K2` =
//! secondary conic constant (negative for Cassegrain/RC):
//!
//! ```text
//! |W131_δ| = (1/8)·(y/F)³·(m−1)²·[(m+1) − (m−1)·K2] · δ   (per mm decenter)
//! |W131_τ| = (1/8)·(y/F)³·(m−1)²·(m+1)·R2 · τ             (per rad tilt, K2-free)
//! ```
//!
//! `W131` is the Seidel coma wavefront coefficient (mm) referenced to the
//! unit pupil. The tilt form is K2-free — a decentered secondary "sees"
//! the K2-dependent asphericity of its own surface differently than a
//! tilted one, per the Scaduto/Wilson derivation.
//!
//! **Coma-free pivot.** The ratio of the two coefficients gives the axial
//! point about which the secondary can be rotated (tilted) with NO
//! induced third-order coma — the ray-trace-verified coefficient ratio
//! (NOT Scaduto's Eq. 28 as transcribed, which is dimensionally
//! inconsistent — yields mm², flagged in [[v2-tolerance-collimation]] and
//! not used here):
//!
//! ```text
//! Z_CFP = (m+1)·R2 / [(m+1) − (m−1)·K2]     (mm, from the secondary vertex,
//!                                             center-of-curvature side)
//! ```
//!
//! `K2 = 0` ⇒ `Z_CFP = R2` exactly — a sphere pivoted about its own center
//! of curvature is invariant, an exact self-check the formula passes
//! (magnitude AND side), see [[v2-tolerance-collimation]].
//!
//! **Chunk VC1 scope:** standalone module + `compute()` free function
//! only, per [[formula-validation-v2]] Phase C sign-off. Wiring into
//! `fabrication::compute`'s dispatch match and adding a `collimation`
//! field to `FabricationReport` (`fabrication/mod.rs`/`dispatch.rs`) are
//! explicitly out of scope for this chunk — same deferred-wiring
//! precedent already used by Chunks V4/V6/V7/VB1/VB3 (this file's brief
//! restricts edits to `collimation.rs` only; `mod.rs`/`dispatch.rs` are
//! untouched here, matching `dispatch.rs`'s own doc comment which still
//! lists `collimation`/#5 as Phase C "BLOCKED, later still").

/// One arcsecond in radians: `π / (180 × 3600)`.
const ARCSEC_TO_RAD: f64 = std::f64::consts::PI / (180.0 * 3600.0);

/// Secondary decenter/tilt collimation-sensitivity result (#5, C1
/// VERIFIED). Matches [[v2-tolerance-collimation]]'s `CollimationSpec`
/// sketch, extended with `coma_free_pivot_mm` and `notes` per this
/// chunk's brief.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CollimationSpec {
    /// Seidel coma wavefront coefficient per mm of secondary decenter,
    /// expressed in waves of `wavelength_nm` (Wilson/Scaduto coefficient,
    /// K2-dependent).
    pub coma_per_mm_decenter_waves: f64,
    /// Seidel coma wavefront coefficient per arcsecond of secondary tilt,
    /// expressed in waves of `wavelength_nm` (K2-free).
    pub coma_per_arcsec_tilt_waves: f64,
    /// Allowable secondary decenter (mm) keeping induced coma below the
    /// λ/4 diffraction limit: `wavelength_nm/4 / (coma_per_mm_decenter_waves
    /// in wavefront-length units)`.
    pub allowable_decenter_mm: f64,
    /// Allowable secondary tilt (arcsec) keeping induced coma below the
    /// λ/4 diffraction limit.
    pub allowable_tilt_arcsec: f64,
    /// Coma-free pivot distance (mm) from the secondary vertex,
    /// center-of-curvature side — rotating the secondary about this point
    /// induces no third-order coma.
    pub coma_free_pivot_mm: f64,
    /// Source citation, echoed verbatim per [[v2-tolerance-collimation]]'s
    /// `CollimationSpec.source` field.
    pub source: String,
    pub notes: Vec<String>,
}

/// Seidel coma wavefront coefficient `|W131_δ|` (mm of wavefront error per
/// mm of secondary decenter):
///
/// `(1/8)·(y/F)³·(m−1)²·[(m+1) − (m−1)·K2]`
///
/// `y_mm` = aperture semi-diameter, `f_mm` = system focal length, `m` =
/// secondary magnification, `k2` = secondary conic constant (negative for
/// Cassegrain/RC — the bracket `(m+1) − (m−1)·K2` is then > 0).
pub fn coma_per_mm_decenter(y_mm: f64, f_mm: f64, m: f64, k2: f64) -> f64 {
    (1.0 / 8.0) * (y_mm / f_mm).powi(3) * (m - 1.0).powi(2) * ((m + 1.0) - (m - 1.0) * k2)
}

/// Seidel coma wavefront coefficient `|W131_τ|` (mm of wavefront error per
/// ARCSECOND of secondary tilt, K2-free):
///
/// `(1/8)·(y/F)³·(m−1)²·(m+1)·R2 · (1 arcsec in radians)`
///
/// `y_mm` = aperture semi-diameter, `f_mm` = system focal length, `m` =
/// secondary magnification, `r2_mm` = secondary radius of curvature
/// magnitude. The locked formula gives the coefficient per RADIAN of
/// tilt; this function converts to per-ARCSECOND internally
/// (`1″ = π/(180×3600) rad`) so the result is directly usable at
/// arcsecond scale, matching the acceptance example's units.
pub fn coma_per_arcsec_tilt(y_mm: f64, f_mm: f64, m: f64, r2_mm: f64) -> f64 {
    let per_radian = (1.0 / 8.0) * (y_mm / f_mm).powi(3) * (m - 1.0).powi(2) * (m + 1.0) * r2_mm;
    per_radian * ARCSEC_TO_RAD
}

/// Coma-free pivot distance (mm) from the secondary vertex,
/// center-of-curvature side: `Z_CFP = (m+1)·R2 / [(m+1) − (m−1)·K2]`.
///
/// This is the ray-trace-verified coefficient ratio
/// ([[v2-tolerance-collimation]] "Locked formulas") — NOT Scaduto's Eq. 28
/// as transcribed in the source paper, which the validation flags as
/// dimensionally inconsistent (yields mm²) and explicitly does not use.
/// `k2 = 0.0` (spherical secondary) reproduces `Z_CFP = R2` exactly, the
/// documented self-check (a sphere pivoted about its own center of
/// curvature is invariant).
pub fn coma_free_pivot_mm(m: f64, r2_mm: f64, k2: f64) -> f64 {
    (m + 1.0) * r2_mm / ((m + 1.0) - (m - 1.0) * k2)
}

/// Compute the full `CollimationSpec`: coma-sensitivity coefficients
/// (per mm decenter, per arcsec tilt), the λ/4-budget allowable decenter
/// and tilt, and the coma-free pivot distance, for a two-mirror system.
///
/// `aperture_d_mm` = full aperture diameter (`y = D/2` internally),
/// `system_focal_length_f_mm` = system focal length, `secondary_magnification_m`
/// = secondary magnification `m`, `r2_mm` = secondary radius of curvature
/// magnitude, `k2` = secondary conic constant, `wavelength_nm` = design
/// wavelength (nm) — the λ/4 budget is `wavelength_nm / 4` converted to mm.
pub fn compute(
    aperture_d_mm: f64,
    system_focal_length_f_mm: f64,
    secondary_magnification_m: f64,
    r2_mm: f64,
    k2: f64,
    wavelength_nm: f64,
) -> CollimationSpec {
    let y_mm = aperture_d_mm / 2.0;
    let wavelength_mm = wavelength_nm * 1e-6;
    let quarter_wave_mm = wavelength_mm / 4.0;

    let w_delta_per_mm = coma_per_mm_decenter(y_mm, system_focal_length_f_mm, secondary_magnification_m, k2);
    let w_tau_per_arcsec = coma_per_arcsec_tilt(y_mm, system_focal_length_f_mm, secondary_magnification_m, r2_mm);

    let coma_per_mm_decenter_waves = w_delta_per_mm / wavelength_mm;
    let coma_per_arcsec_tilt_waves = w_tau_per_arcsec / wavelength_mm;

    let allowable_decenter_mm = quarter_wave_mm / w_delta_per_mm;
    let allowable_tilt_arcsec = quarter_wave_mm / w_tau_per_arcsec;

    let coma_free_pivot_mm_val = coma_free_pivot_mm(secondary_magnification_m, r2_mm, k2);

    let notes = vec![
        "coma-free pivot is measured from the secondary vertex, center-of-curvature side; \
         rotating the secondary about this point induces no third-order coma"
            .to_string(),
    ];

    CollimationSpec {
        coma_per_mm_decenter_waves,
        coma_per_arcsec_tilt_waves,
        allowable_decenter_mm,
        allowable_tilt_arcsec,
        coma_free_pivot_mm: coma_free_pivot_mm_val,
        source: "Scaduto et al. 2013 Opt. Express 21:6851 (Wilson RTO I ch.3)".to_string(),
        notes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A2 RC worked example ([[formula-validation-v2]] C1 VERIFIED,
    /// [[v2-tolerance-collimation]] "Worked numbers"): D=200mm, F=1600mm,
    /// m=8/3, R2=654.5455mm, K2=−6.5878621, λ=550nm.
    fn a2_rc() -> CollimationSpec {
        compute(200.0, 1600.0, 8.0 / 3.0, 654.5455, -6.5878621, 550.0)
    }

    #[test]
    fn coma_per_mm_decenter_matches_worked_example() {
        let spec = a2_rc();
        assert!(
            (spec.coma_per_mm_decenter_waves - 2.257).abs() < 0.01,
            "got {}",
            spec.coma_per_mm_decenter_waves
        );
    }

    #[test]
    fn coma_per_arcsec_tilt_matches_worked_example() {
        let spec = a2_rc();
        assert!(
            (spec.coma_per_arcsec_tilt_waves - 1.793e-3).abs() < 0.0001,
            "got {}",
            spec.coma_per_arcsec_tilt_waves
        );
    }

    #[test]
    fn allowable_decenter_matches_worked_example() {
        let spec = a2_rc();
        assert!(
            (spec.allowable_decenter_mm - 0.111).abs() < 0.001,
            "got {}",
            spec.allowable_decenter_mm
        );
    }

    #[test]
    fn allowable_tilt_matches_worked_example() {
        let spec = a2_rc();
        assert!(
            (spec.allowable_tilt_arcsec - 139.0).abs() < 1.0,
            "got {}",
            spec.allowable_tilt_arcsec
        );
    }

    #[test]
    fn coma_free_pivot_matches_worked_example() {
        let spec = a2_rc();
        assert!(
            (spec.coma_free_pivot_mm - 163.86).abs() < 0.1,
            "got {}",
            spec.coma_free_pivot_mm
        );
    }

    /// Self-check documented in [[v2-tolerance-collimation]]: K2 = 0
    /// (spherical secondary) reproduces `Z_CFP = R2` exactly — a sphere
    /// pivoted about its own center of curvature is invariant.
    #[test]
    fn coma_free_pivot_sphere_equals_r2() {
        let z_cfp = coma_free_pivot_mm(8.0 / 3.0, 654.5455, 0.0);
        assert!((z_cfp - 654.5455).abs() < 1e-9, "got {}", z_cfp);
    }
}
