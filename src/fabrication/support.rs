//! Mirror support-sag (gravity print-through) physics (#6,
//! [[v2-support-seidel]] `### N-point edge/back support`) — Chunk VB3,
//! [[task-snapshot-v2-phaseB]]. Closed-form RMS-sag approximation, NOT
//! finite-element ([[constraints]] analytical-only), now VERIFIED as item
//! **S1** in [[formula-validation-v2]].
//!
//! **Chunk VB3 scope:** standalone module + `compute()` free function
//! only. Wiring into `fabrication::compute`'s dispatch match and adding a
//! `support` field to `FabricationReport` (`fabrication/mod.rs`) are
//! explicitly out of scope for this chunk (VB3's delegation brief
//! restricts edits to `support.rs`, `substrate.rs`, and
//! `configs/substrates.toml` only) — left as an open follow-up for the
//! chunk that owns `mod.rs`'s dispatch wiring, same deferred-wiring
//! pattern already used by Chunks V4/V6/V7/VB1.
//!
//! **Reference:** Timoshenko & Woinowsky-Krieger, *Theory of Plates and
//! Shells*, 2nd ed., ch. 3 (circular-plate deflection under uniformly
//! distributed self-weight load) — the standard reference Couder's and
//! Malacara's (*Optical Shop Testing*, 3rd ed., ch. 12, "Mirror
//! supports") ATM/telescope-mirror support formulas are themselves
//! derived from.
//!
//! **Locked formula (S1 VERIFIED, [[formula-validation-v2]]):**
//!
//! ```text
//! δ_rms = C · (ρ·g/E) · D⁴ / (N² · t²)
//! ```
//!
//! where `D` = full mirror diameter (m), `t` = thickness (m), `ρ` =
//! density (kg/m³), `E` = Young's modulus (Pa), `g` = 9.81 m/s², `N` =
//! number of discrete support points, `C` = dimensionless support-scheme
//! constant. **`D` is the actual mirror diameter — do NOT substitute an
//! effective `D_eff ≈ D/√N`**, which would double-count the `N` scaling
//! already carried by the explicit `N²` divisor (the validation's
//! corrected-form note: `D_eff⁴/N² = D⁴/N⁴ ≠ D⁴/N²`).
//!
//! Dimensional check (SI): `[ρ·g/E] = (kg/m³)(m/s²)/(Pa) = 1/m`; then
//! `(1/m)·m⁴/m² = m`. Confirmed: yields meters.
//!
//! **Per-support-scheme `C` values** (S1 VERIFIED table):
//!
//! | Support scheme | C |
//! |---|---|
//! | 3-point edge/triangle | 0.0975 |
//! | 6-point back | 0.04875 |
//! | 9-point back | 0.04875 |
//! | whiffletree (V-10) | 0.04875 (effective N = physical pivots / 2) |
//! | full_disk (V-10) | 0.04875 (sag reported as 0.0 — continuous bed) |
//!
//! Infallible arithmetic: `compute` validates only the support-scheme
//! name (returns `OptError::InvalidValue` for an unrecognized scheme,
//! reusing the existing generic validation-style error variant rather
//! than adding a new one-off variant to `error.rs` for a single call
//! site). All numeric inputs (`diameter_mm`, `thickness_mm`, density,
//! Young's modulus, `n_points`) are already range-validated upstream by
//! `request_validate.rs`/the substrate catalog before reaching this
//! module, mirroring every other Phase A/B subsystem's infallibility
//! convention (`substrate_mass.rs`, `tolerance/mod.rs`).

use crate::error::OptError;
use crate::substrate::Substrate;

/// Support-sag result for one mirror + support-scheme combination
/// ([[v2-support-seidel]] `### SupportSpec`).
///
/// **V-11 fix:** this file's field names (`support_scheme`, `n_points`,
/// `sag_nm`, `sag_pct_of_lambda_8`, `notes`) do NOT match
/// [[v2-support-seidel]]'s published sketch (`scheme`, `support_points`,
/// `rms_sag_nm`, `sag_waves`, `within_quarter_wave`, `provisional`) —
/// VB3's doc comment previously claimed a cross-check that was false. The
/// renamed fields are a defensible, self-consistent naming choice (kept
/// as-is here to avoid a churn-only rename across every call site/test in
/// this already-shipped module); the doc's `within_quarter_wave`
/// pass/fail *behavior* was a genuinely dropped deliverable, not just a
/// naming difference, and is added below.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SupportSpec {
    pub support_scheme: String,
    pub n_points: u32,
    pub sag_nm: f64,
    /// `sag_nm` expressed as a percentage of the λ/8 surface P-V budget
    /// at 550 nm (68.75 nm, Phase A's already-verified tolerance) — a
    /// fixed reference wavelength for a quick "how much of my budget did
    /// gravity sag eat" comparison, independent of any per-request
    /// `wavelength_nm`.
    pub sag_pct_of_lambda_8: f64,
    /// V-11 addition: `true` when `sag_nm` stays within the λ/4 (550 nm)
    /// wavefront-equivalent budget (137.5 nm) — the same fixed 550 nm
    /// reference as `sag_pct_of_lambda_8`, at the λ/4 (not λ/8) point
    /// ([[v2-support-seidel]] "`sag_waves`... > λ/4 → warning note").
    pub within_quarter_wave: bool,
    pub notes: Vec<String>,
}

/// Resolve a `support_scheme` name to its dimensionless geometry constant
/// `C` (S1 VERIFIED table, [[formula-validation-v2]]).
///
/// **V-10 addition:** `whiffletree` and `full_disk`
/// ([[v2-support-seidel]] `### Support schemes`) — both share the
/// well-distributed-back-support baseline `C = 0.04875` (same as
/// `6_point`/`9_point`); `compute` below applies the schemes' distinct
/// *effective-N*/near-zero-sag behavior on top of this shared constant.
fn scheme_constant(support_scheme: &str) -> Result<f64, OptError> {
    match support_scheme {
        "3_point" | "3-point" | "edge_triangle" => Ok(0.0975),
        "6_point" | "6-point" => Ok(0.04875),
        "9_point" | "9-point" => Ok(0.04875),
        "whiffletree" => Ok(0.04875),
        "full_disk" => Ok(0.04875),
        other => Err(OptError::InvalidValue {
            field: "support_scheme".to_string(),
            reason: format!(
                "unrecognized support scheme '{other}' (expected one of: 3_point, 6_point, \
                 9_point, whiffletree, full_disk)"
            ),
        }),
    }
}

/// The Rayleigh λ/4 wavefront-equivalent tolerance at the fixed reference
/// wavelength 550 nm, in nanometers — `550.0 / 4.0 = 137.5` nm. Used only
/// for `within_quarter_wave`'s pass/fail check (V-11), same fixed-550nm
/// convention as `LAMBDA_8_REFERENCE_NM` below.
const LAMBDA_4_REFERENCE_NM: f64 = 550.0 / 4.0;

/// Compute the RMS gravity-sag print-through, in nanometers, for a
/// circular mirror of `diameter_mm`/`thickness_mm` on `n_points` discrete
/// supports arranged per `support_scheme_constant_c` (the dimensionless
/// geometry constant `C`).
///
/// Locked formula (S1 VERIFIED): `δ_rms = C · (ρg/E) · D⁴ / (N²·t²)`, all
/// SI internally (mm → m, GPa → Pa via ×1e9), converted to nm for the
/// return value (`δ_rms[m] × 1e9`).
pub fn support_sag_nm(
    diameter_mm: f64,
    thickness_mm: f64,
    density_kg_m3: f64,
    youngs_modulus_gpa: f64,
    n_points: u32,
    support_scheme_constant_c: f64,
) -> f64 {
    const G_M_PER_S2: f64 = 9.81;

    let diameter_m = diameter_mm / 1000.0;
    let thickness_m = thickness_mm / 1000.0;
    let youngs_modulus_pa = youngs_modulus_gpa * 1e9;
    let n = f64::from(n_points);

    let rho_g_over_e = density_kg_m3 * G_M_PER_S2 / youngs_modulus_pa;
    let sag_m = support_scheme_constant_c * rho_g_over_e * diameter_m.powi(4)
        / (n * n * thickness_m * thickness_m);

    sag_m * 1e9
}

/// The Phase A λ/8 surface P-V tolerance at the fixed reference
/// wavelength 550 nm, in nanometers — `550.0 / 8.0 = 68.75` nm. Used only
/// for this subsystem's "percent of budget" comparison, independent of
/// any per-request `wavelength_nm` (`tolerance::compute_phase_a`/
/// `compute_phase_b_conic` compute the request-specific figure
/// separately).
const LAMBDA_8_REFERENCE_NM: f64 = 550.0 / 8.0;

/// Compute the full `SupportSpec` for a mirror of `diameter_mm`/
/// `thickness_mm`, `substrate` (supplies density + Young's modulus), and
/// `support_scheme` (mapped to its `C` constant + validated), with
/// `n_points` discrete support points.
///
/// **V-10 scheme handling:**
/// - `"whiffletree"`: `n_points` is the *physical pivot count* (e.g. an
///   18-point whiffletree); modeled as its *effective* support-point
///   count for sag purposes, `n_points / 2` (an 18-point whiffletree ≈
///   9-point direct support, [[v2-support-seidel]]'s cited convention).
///   `ponytail:` this halving is an approximate ceiling, not a per-
///   triangle FEA — a documented convention, not a literature constant;
///   upgrade path is real whiffletree FEA if a build ever needs it.
/// - `"full_disk"`: continuous bed support — sag is reported as
///   `0.0` nm ("negligible", per [[v2-support-seidel]]), skipping the
///   N-point formula entirely (a continuous bed has no discrete-N sag
///   term to evaluate). `n_points` is ignored for this scheme (echoed
///   back on the spec as given, since it has no physical meaning here).
///
/// Returns `OptError::InvalidValue` for an unrecognized `support_scheme`
/// name — never panics.
pub fn compute(
    diameter_mm: f64,
    thickness_mm: f64,
    substrate: &Substrate,
    support_scheme: &str,
    n_points: u32,
) -> Result<SupportSpec, OptError> {
    let c = scheme_constant(support_scheme)?;
    let density_kg_m3 = substrate.density_g_cm3 * 1000.0;

    let (sag_nm, mut notes) = if support_scheme == "full_disk" {
        (
            0.0,
            vec!["negligible (full-disk support) — continuous bed, no discrete-N sag term"
                .to_string()],
        )
    } else if support_scheme == "whiffletree" {
        let effective_n_points = (n_points / 2).max(1);
        let sag = support_sag_nm(
            diameter_mm,
            thickness_mm,
            density_kg_m3,
            substrate.youngs_modulus_gpa,
            effective_n_points,
            c,
        );
        (
            sag,
            vec![format!(
                "whiffletree modeled as its effective support-point count ({effective_n_points}), \
                 not per-triangle FEA (ponytail: {n_points}-point whiffletree ~= {effective_n_points}-point \
                 direct support for sag purposes)"
            )],
        )
    } else {
        let sag = support_sag_nm(
            diameter_mm,
            thickness_mm,
            density_kg_m3,
            substrate.youngs_modulus_gpa,
            n_points,
            c,
        );
        (sag, vec![])
    };

    let sag_pct_of_lambda_8 = 100.0 * sag_nm / LAMBDA_8_REFERENCE_NM;
    let within_quarter_wave = sag_nm <= LAMBDA_4_REFERENCE_NM;

    notes.push(format!(
        "support sag {sag_nm:.3} nm is {sag_pct_of_lambda_8:.2}% of the λ/8 (550nm) surface tolerance budget"
    ));

    Ok(SupportSpec {
        support_scheme: support_scheme.to_string(),
        n_points,
        sag_nm,
        sag_pct_of_lambda_8,
        within_quarter_wave,
        notes,
    })
}

#[cfg(test)]
mod tests;
