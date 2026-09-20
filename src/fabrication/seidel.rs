//! Off-axis Seidel aberration budget (#7, [[v2-support-seidel]] `## #7 —
//! Off-axis Seidel aberration budget`) — Chunk VC2, [[formula-validation-v2]]
//! item **S2** (VERIFIED 2026-09-20). Third-order Seidel sums (coma,
//! astigmatism, field curvature) for a **given, already-solved** two-mirror
//! (Cassegrain-family) telescope — aberration *evaluation* of a fixed
//! prescription, NOT optimization ([[architecture-v2]] "kept strictly
//! analytical").
//!
//! **Reference:** Welford, *Aberrations of Optical Systems*, ch.7 §7.2
//! (Seidel surface-contribution sums) + ch.8 (aspheric/conic terms,
//! stop-shift); cross-checked against Schroeder, *Astronomical Optics*
//! 2nd ed., ch.3 + §6.2 (two-mirror coma theorem).
//!
//! **Locked formulas** (see [[v2-support-seidel]] "Locked formulas" for
//! the full derivation, sign convention, and the mandatory self-check
//! this module reproduces):
//!
//! ```text
//! S_I   = -Sum A^2*h*D(u/n)    + Sum (n'-n)*K*c^3*h^4
//! S_II  = -Sum A*Abar*h*D(u/n) + Sum (n'-n)*K*c^3*hbar*h^3
//! S_III = -Sum Abar^2*h*D(u/n) + Sum (n'-n)*K*c^3*hbar^2*h^2
//! S_IV  = -Sum H^2*c*D(1/n)                        (no conic term)
//! ```
//!
//! The paraxial trace itself (2 mirrors, marginal + chief ray, stop at
//! primary) lives in the sibling `seidel_trace` submodule — kept separate
//! to stay under this file's 300-line budget.
//!
//! **Chunk VC2 scope:** standalone module + `compute()` free function,
//! per [[formula-validation-v2]] Phase C sign-off (parallel to Chunk
//! VC1's `collimation.rs`).
//!
//! **Wiring is complete.** `dispatch_seidel` in
//! `fabrication/dispatch/phase_c.rs` calls `seidel::compute` and is
//! wired into the top-level dispatch in `fabrication/dispatch/mod.rs`,
//! which populates `FabricationReport`'s `seidel` field
//! (`fabrication/mod.rs`).

mod seidel_trace;

use seidel_trace::{seidel_sums, trace_two_mirror};

/// One radian in arcseconds: `180*3600/pi`.
const RAD_TO_ARCSEC: f64 = 206_264.806_247_096_36;

/// Third-order Seidel aberration result for a two-mirror system at a
/// given field angle ([[v2-support-seidel]] `### SeidelSpec`).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SeidelSpec {
    pub field_of_view_deg: f64,
    /// S_II — Seidel coma sum (mm). ~0 for an aplanatic (RC) design;
    /// clearly non-zero for a classical Cassegrain.
    pub coma_s2: f64,
    /// S_III — Seidel astigmatism sum (mm).
    pub astigmatism_s3: f64,
    /// S_IV — Seidel field-curvature (Petzval) sum (mm).
    pub field_curvature_s4: f64,
    /// Combined edge-of-field blur estimate (arcsec) — the headline
    /// dominant-term figure (coma when S_II is significant, astigmatism
    /// when the design is coma-corrected/aplanatic).
    pub edge_blur_arcsec: f64,
    /// Headline design classification: `"aplanatic (coma-nulled)"` when
    /// `|S_II|` is negligible relative to the astigmatism term, else
    /// `"coma-dominated"`.
    pub conic_mode: Option<String>,
    pub provisional: bool,
    pub source: String,
}

/// Convert Seidel sums + the paraxial trace's exit `(n', u')` and EFL
/// into arcsecond blur figures: angular tangential coma
/// `ATC = 3*S_II/(2*n'*u'*f)` and astigmatism (medial) blur diameter
/// `= S_III/(n'*u'*f)`, both converted radians -> arcsec.
fn blur_components_arcsec(
    s2: f64,
    s3: f64,
    exit_n_prime: f64,
    exit_u_prime: f64,
    efl_mm: f64,
) -> (f64, f64) {
    let n_prime_u_prime = exit_n_prime * exit_u_prime;
    let atc_rad = 3.0 * s2 / (2.0 * n_prime_u_prime * efl_mm);
    let astig_rad = s3 / (n_prime_u_prime * efl_mm);
    (atc_rad.abs() * RAD_TO_ARCSEC, astig_rad.abs() * RAD_TO_ARCSEC)
}

/// Compute the full `SeidelSpec` for a two-mirror system.
///
/// - `aperture_d_mm`: full primary aperture `D`.
/// - `system_focal_length_f_mm`: system EFL `F` (used for the blur
///   conversion's `f`; the trace itself derives its own exit `u'`
///   independently and does not need `F` as an input to the sums).
/// - `r1_mm`/`r2_mm`: primary/secondary radius-of-curvature
///   **magnitudes** (v1's existing positive-magnitude convention — the
///   locked `c=1/R, R<0 for both mirrors` sign is applied internally).
/// - `k1`/`k2`: primary/secondary conic constants (signed, as stored).
/// - `mirror_separation_d_mm`: primary-to-secondary separation
///   **magnitude** (v1 convention; the locked `d=-|separation|`
///   negative-along-travel sign is applied internally).
/// - `field_diam_mm`: full fully-illuminated field diameter (`2h` at the
///   focal plane) — same convention as `baffle::compute`'s
///   `field_diam_mm`, converted internally to a half-field angle
///   `theta = atan((field_diam_mm/2)/F)` for the chief-ray trace.
/// - `wavelength_nm`: unused by the third-order geometric sums
///   themselves (Seidel sums are wavelength-independent to this order)
///   but accepted per the signature the KB doc's worked example
///   parameterizes with, for API symmetry with `collimation::compute`
///   and to leave room for a future diffraction-limit comparison; kept
///   as an explicit parameter rather than silently dropped input.
#[allow(clippy::too_many_arguments)]
pub fn compute(
    aperture_d_mm: f64,
    system_focal_length_f_mm: f64,
    r1_mm: f64,
    r2_mm: f64,
    k1: f64,
    k2: f64,
    mirror_separation_d_mm: f64,
    field_diam_mm: f64,
    _wavelength_nm: f64,
) -> SeidelSpec {
    let h_field = field_diam_mm / 2.0;
    let theta_rad = (h_field / system_focal_length_f_mm).atan();
    let field_of_view_deg = 2.0 * theta_rad.to_degrees();

    let trace = trace_two_mirror(
        aperture_d_mm,
        r1_mm,
        r2_mm,
        mirror_separation_d_mm,
        k1,
        k2,
        theta_rad,
    );
    let (_s1, s2, s3, s4) = seidel_sums(&trace);

    let (atc_arcsec, astig_arcsec) = blur_components_arcsec(
        s2,
        s3,
        trace.exit_n_prime,
        trace.exit_u_prime,
        system_focal_length_f_mm,
    );

    // Headline classification + dominant-term blur: an aplanatic (RC)
    // design nulls coma, leaving astigmatism as the reported figure; a
    // classical (non-aplanatic) design is coma-dominated and that term
    // is reported instead ([[v2-support-seidel]] worked example: RC
    // 0.63" astig-dominated vs classical 2.64" coma-dominated).
    let (conic_mode, edge_blur_arcsec) = if atc_arcsec > astig_arcsec {
        (Some("coma-dominated".to_string()), atc_arcsec)
    } else {
        (Some("aplanatic (coma-nulled)".to_string()), astig_arcsec)
    };

    SeidelSpec {
        field_of_view_deg,
        coma_s2: s2,
        astigmatism_s3: s3,
        field_curvature_s4: s4,
        edge_blur_arcsec,
        conic_mode,
        provisional: false,
        source: "Welford ch.7 §7.2 + ch.8".to_string(),
    }
}

#[cfg(test)]
mod tests;
