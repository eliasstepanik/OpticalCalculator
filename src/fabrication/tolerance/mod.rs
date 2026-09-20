//! Surface-figure tolerance physics (#4) — Chunk V7 shipped the λ/8
//! surface / λ/4 wavefront Rayleigh criterion ([[v2-tolerance-collimation]]
//! `### Surface-figure tolerance`); **Chunk VB2 adds the conic-constant
//! tolerance formula** ([[v2-tolerance-collimation]] `### Conic-constant
//! tolerance`), now VERIFIED as item **T1** in [[formula-validation-v2]].
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
//! **Conic-constant tolerance (Chunk VB2, T1 VERIFIED).**
//! [[formula-validation-v2]] T1 found the doc's provisional
//! `ΔK_tol = 2·λ·R³/y⁴` was **wrong by a factor of 2** — it omitted the
//! double-pass reflection doubling already used above for the λ/8
//! surface ⇔ λ/4 wavefront pair. The corrected, VERIFIED closed form,
//! derived from the conic sag Taylor expansion
//! (`z(r) ≈ r²/(2R) + (1+K)·r⁴/(8R³)`) and cross-checked against the
//! independently published Texereau/Suiter `D⁴/(2048·F³)`
//! sphere-vs-paraboloid criterion (both forms agree to 6 significant
//! figures for D=150mm, F=1200mm f/8 — see [[formula-validation-v2]] T1):
//!
//! ```text
//! ΔW = ΔK · y⁴ / (4·R³)     (wavefront P-V error from a conic-constant
//!                            error ΔK, double-pass reflection included)
//! ```
//!
//! Setting `ΔW = λ/4` (the same Rayleigh criterion as the surface-figure
//! tolerance above) and solving for the allowable `ΔK`:
//!
//! ```text
//! ΔK_tol = 4 · λ · R³ / y⁴
//! ```
//!
//! **`4·λ·R³/y⁴` is the correct, VERIFIED constant — NOT the stale
//! `2·λ·R³/y⁴` form still written in [[v2-tolerance-collimation]]'s
//! pre-validation draft.** `allowable_conic_tolerance` below implements
//! only the corrected form.
//!
//! Any collimation / tilt-coma coefficient (#5, item **C1**) is
//! implemented separately in `fabrication::collimation` — not this module.
//!
//! **Wiring is complete.** `compute_phase_b_conic` is called from
//! `fabrication::dispatch::mod`'s top-level dispatch match
//! (`fabrication/dispatch/mod.rs`), which populates `FabricationReport`'s
//! `tolerance` field (`fabrication/mod.rs`).

/// Surface-figure + conic-constant tolerance result: the Rayleigh λ/8
/// surface P-V figure tolerance (Phase A, Chunk V7), its λ/4
/// reflected-wavefront counterpart, and the per-surface conic-constant
/// tolerance (Phase B, Chunk VB2, T1 VERIFIED) for a mirror system at
/// `wavelength_nm`.
///
/// Matches [[v2-tolerance-collimation]]'s full `ToleranceSpec` sketch
/// (Phase A + Phase B fields together in one struct, not a nested type —
/// simpler for this small, flat field set; see `compute_phase_b_conic`'s
/// doc comment for the naming rationale). `secondary_conic_tolerance`/
/// `secondary_conic_tolerance_fractional` are `None` for single-mirror
/// (non-Cassegrain-family) designs.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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
    /// Allowable conic-constant error `ΔK_tol = 4·λ·R³/y⁴` for the primary
    /// mirror at the λ/4 wavefront criterion (T1 VERIFIED,
    /// [[formula-validation-v2]]). Dimensionless (same units as `K`
    /// itself).
    pub primary_conic_tolerance: f64,
    /// The same fraction expressed relative to the primary's own conic
    /// constant magnitude, `ΔK_tol / |K1|` — `None` when `k1 == 0.0`
    /// (undefined fraction; a spherical primary has no conic to be
    /// fractionally tolerant of).
    pub primary_conic_tolerance_fractional: Option<f64>,
    /// Allowable conic-constant error for the secondary mirror
    /// (Cassegrain-family designs only) — `None` for single-mirror
    /// designs with no secondary conic.
    pub secondary_conic_tolerance: Option<f64>,
    /// `ΔK_tol / |K2|` for the secondary — `None` when there is no
    /// secondary or `k2 == 0.0`.
    pub secondary_conic_tolerance_fractional: Option<f64>,
    /// V-11 addition: `"rc" | "classical"` — echoes `DesignDef.conic_mode`
    /// for the report, so the "RC vs classical sensitivity ... must be
    /// quantitative" caveat ([[v2-tolerance-collimation]]) can be read
    /// directly off a single report rather than requiring the reader to
    /// cross-reference `configs/designs.toml`. `None` for single-mirror
    /// designs with no conic-mode concept. Carries the string through for
    /// reporting only — no RC-vs-classical comparison LOGIC is implemented
    /// here (that would require computing both variants for the same
    /// geometry, which is out of scope; see [[formula-validation-v2]] T1's
    /// worked-example table for the actual comparison numbers).
    pub conic_mode: Option<String>,
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

/// Allowable conic-constant tolerance `ΔK_tol = 4·λ·R³/y⁴` for a mirror
/// surface at `wavelength_nm`, radius of curvature `radius_of_curvature_mm`
/// (`R`), and semi-aperture `semi_aperture_mm` (`y`).
///
/// Formula (T1 VERIFIED, [[formula-validation-v2]] — see this module's doc
/// comment for the full derivation and the Texereau/Suiter cross-check):
/// `ΔK_tol = 4 · λ_mm · R³ / y⁴`, where `λ_mm = wavelength_nm × 1e-6`
/// converts the wavelength from nanometers to millimeters so it is
/// dimensionally consistent with `R`/`y` (both millimeters), yielding a
/// dimensionless `ΔK`.
pub fn allowable_conic_tolerance(
    wavelength_nm: f64,
    radius_of_curvature_mm: f64,
    semi_aperture_mm: f64,
) -> f64 {
    let wavelength_mm = wavelength_nm * 1e-6;
    4.0 * wavelength_mm * radius_of_curvature_mm.powi(3) / semi_aperture_mm.powi(4)
}

/// Compute the Phase-A `ToleranceSpec` subset (λ/8 surface + λ/4
/// wavefront) for `wavelength_nm`, with the conic-tolerance fields all
/// `None`/`0.0` (no conic geometry available). Infallible: `wavelength_nm`
/// is range-validated upstream (`request_validate.rs`) before a `Request`
/// reaches this module.
///
/// **Superseded by `compute_phase_b_conic` for any design that has conic
/// geometry** (i.e. every current design — Newtonian, Cassegrain-family,
/// refractor pipelines all carry at least a primary conic). Retained
/// (rather than removed) only so a future single-surface-only caller with
/// no radius/aperture data on hand still has a Phase-A-only entry point;
/// `fabrication::compute`'s dispatch (`fabrication/mod.rs`, out of scope
/// for this chunk) decides which of the two to call.
pub fn compute_phase_a(wavelength_nm: f64) -> ToleranceSpec {
    ToleranceSpec {
        wavelength_nm,
        surface_pv_tol_nm: wavelength_nm / 8.0,
        surface_pv_tol_waves: 0.125,
        wavefront_pv_tol_nm: wavelength_nm / 4.0,
        wavefront_pv_tol_waves: 0.25,
        primary_conic_tolerance: 0.0,
        primary_conic_tolerance_fractional: None,
        secondary_conic_tolerance: None,
        secondary_conic_tolerance_fractional: None,
        conic_mode: None,
    }
}

/// Compute the full Phase A + Phase B `ToleranceSpec`: the λ/8 surface /
/// λ/4 wavefront Rayleigh criterion (Phase A, Chunk V7) plus the
/// per-surface conic-constant tolerance (Phase B, Chunk VB2, T1 VERIFIED).
///
/// Consumes exactly the conic geometry available on `output::CassegrainSpec`
/// (`k1_primary_conic`, `k2_secondary_conic`, `r1_mm`, `r2_mm`) plus the
/// semi-apertures `y1_mm`/`y2_mm` (half the clear aperture at each
/// surface — not currently a `CassegrainSpec` field, so the caller
/// supplies it; for a single-mirror design pass `k2`/`r2_mm`/`y2_mm` as
/// `None`). `k1`/`r1_mm`/`y1_mm` are always required (every design has at
/// least a primary conic).
///
/// **Naming choice:** kept as a separate function alongside `compute_phase_a`
/// rather than renaming/replacing it, because `compute_phase_a`'s single
/// `wavelength_nm` signature has no conic geometry to consume — merging the
/// two would force every existing Phase-A-only call site to fabricate
/// `k1`/`r1_mm`/`y1_mm` values it does not have. This keeps both entry
/// points honest about what data they actually require.
///
/// `conic_mode` (V-11 addition) is echoed onto the returned spec's
/// `conic_mode` field verbatim (`DesignDef.conic_mode`, `"rc"` |
/// `"classical"` | `None` for single-mirror designs) — reporting only, no
/// comparison logic computed here.
pub fn compute_phase_b_conic(
    wavelength_nm: f64,
    k1: f64,
    r1_mm: f64,
    y1_mm: f64,
    k2: Option<f64>,
    r2_mm: Option<f64>,
    y2_mm: Option<f64>,
    conic_mode: Option<String>,
) -> ToleranceSpec {
    let primary_conic_tolerance = allowable_conic_tolerance(wavelength_nm, r1_mm, y1_mm);
    let primary_conic_tolerance_fractional = if k1 == 0.0 {
        None
    } else {
        Some(primary_conic_tolerance / k1.abs())
    };

    let (secondary_conic_tolerance, secondary_conic_tolerance_fractional) =
        match (k2, r2_mm, y2_mm) {
            (Some(k2), Some(r2_mm), Some(y2_mm)) => {
                let tol = allowable_conic_tolerance(wavelength_nm, r2_mm, y2_mm);
                let frac = if k2 == 0.0 { None } else { Some(tol / k2.abs()) };
                (Some(tol), frac)
            }
            _ => (None, None),
        };

    ToleranceSpec {
        wavelength_nm,
        surface_pv_tol_nm: wavelength_nm / 8.0,
        surface_pv_tol_waves: 0.125,
        wavefront_pv_tol_nm: wavelength_nm / 4.0,
        wavefront_pv_tol_waves: 0.25,
        primary_conic_tolerance,
        primary_conic_tolerance_fractional,
        secondary_conic_tolerance,
        secondary_conic_tolerance_fractional,
        conic_mode,
    }
}

#[cfg(test)]
mod tests;
