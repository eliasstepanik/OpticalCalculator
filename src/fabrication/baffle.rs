//! Baffle & anti-vignetting geometry (#3, [[v2-baffle-vignetting]]) —
//! Chunk VB1, [[formula-validation-v2]] item B1 (VERIFIED). Cassegrain-
//! family primary-hole / secondary-baffle-tube diameter profile, closed
//! form, no ray tracing.
//!
//! **B1 VERIFIED formula.** The naive tangent-line construction anchored
//! at the primary's physical plane was falsified against an exact
//! paraxial ray trace ([[formula-validation-v2]] B1). The correct
//! construction treats the final f/system cone as converging from a
//! virtual full-aperture disk of diameter `D` one system focal length `F`
//! from the focal plane. With `h` = fully-illuminated field radius at the
//! focal plane and `s` = axial distance from the focal plane toward the
//! primary:
//!
//! ```text
//! Ø_baffle(s) = 2h·(1 − s/F) + D·(s/F)     0 ≤ s ≤ d+b
//! ```
//!
//! Reference: Schroeder, *Astronomical Optics* 2nd ed. §6.5; Rutten & van
//! Venrooij, *Telescope Optics*, ch.17. Cross-validated against an
//! independent exact paraxial ray trace of the A2 RC system
//! ([[formula-validation-v2]] B1) — conservative by a few mm at each
//! position, never vignettes.
//!
//! **h-convention (pinned by B1's resolution, [[v2-baffle-vignetting]]):**
//! reuses v1's existing `Request.field_diam_mm` directly as `2h` for
//! Cassegrain-family designs too, rather than introducing a second,
//! redundant `field_of_view_deg` convention. `compute` below therefore
//! takes `field_diam_mm: Option<f64>` and returns `None` when it is
//! absent — mirrors the graceful "not applicable" pattern used by
//! `fabrication::compute`'s other Option-gated subsystems
//! (`fabrication/mod.rs`) rather than erroring.
//!
//! **`secondary_baffle_tube_length_mm` stays `None`.** B1's resolution is
//! explicit: the two-tangent construction fixes only the diameter profile
//! `Ø(s)` — any tube whose wall stays on/outside that envelope transmits
//! the field unvignetted at ANY length. A true length needs an additional
//! stray-light-closure condition that is not satisfiable with both rims
//! exactly on the minimum envelope for this system; real designs trade
//! secondary-baffle oversize against tube length as a free mechanical
//! choice. This module does not invent a value.
//!
//! **Chunk VB1 scope:** the Cassegrain-family ("primary variant") case
//! only. The Newtonian variant ([[v2-baffle-vignetting]] "Newtonian
//! variant" section — reporting v1's already-computed secondary data
//! rather than duplicating cone math) is scoped OUT of this chunk: even
//! as thin wrapper plumbing it needs a v1 `NewtonianSecondarySpec` input
//! shape decision this brief did not fix, so it is left as a follow-up
//! for a later chunk rather than guessed here.
//!
//! **Wiring (Cassegrain-family case) is complete.** `baffle::compute` is
//! called from `fabrication::dispatch::mod`'s top-level dispatch match
//! (`fabrication/dispatch/mod.rs`), which populates `FabricationReport`'s
//! `baffle` field (`fabrication/mod.rs`).

/// Cassegrain-family baffle/vignetting result (#3,
/// [[v2-baffle-vignetting]] `## BaffleSpec`). `secondary_baffle_tube_length_mm`
/// is always `None` — see module doc comment ("B1 resolution").
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BaffleSpec {
    /// `"cassegrain"` — this chunk's only implemented variant.
    pub kind: String,
    /// `Ø_baffle(s=b)` at the primary's central baffle hole plane, mm.
    pub primary_baffle_hole_diameter_mm: f64,
    /// `Ø_baffle(s=b+d/2)`, the midway diameter, mm — reported alongside
    /// the two endpoints so the profile's near-linear shape is visible.
    pub midway_baffle_diameter_mm: f64,
    /// `Ø_baffle(s=b+d)` at the secondary's plane, mm — this is also the
    /// secondary baffle tube's minimum entrance diameter.
    pub secondary_baffle_tube_diameter_mm: f64,
    /// NOT optics-determined — see module doc comment. Always `None`.
    pub secondary_baffle_tube_length_mm: Option<f64>,
    /// `100 * secondary_baffle_tube_diameter_mm / aperture_d_mm` — the
    /// linear central obstruction if the secondary baffle is built to its
    /// minimum envelope diameter (surfaced, not hidden, per B1's
    /// resolution note).
    pub linear_obstruction_pct: f64,
    /// The fully-illuminated field expressed as a full angular field in
    /// degrees, `2·atan(h/F)`, derived from the linear `field_diam_mm`
    /// input for display — informational only, not a second input
    /// convention (see module doc comment).
    pub fully_illuminated_field_deg: f64,
    /// V-11 addition ([[v2-baffle-vignetting]] `## BaffleSpec` sketch).
    /// Always `true` for this module's model: `compute` sizes the baffle
    /// diameters to the REQUIRED envelope for the given `field_diam_mm`
    /// (an inverse-design tool, not a fixed-baffle-vs-field check), so
    /// the input field is by construction always fully illuminated —
    /// there is no "shrink the field" case to detect for a tool that
    /// computes the baffle FROM the field rather than the reverse.
    pub vignetting_free: bool,
    pub notes: Vec<String>,
}

/// `Ø_baffle(s) = 2h·(1 − s/F) + D·(s/F)` (B1 VERIFIED,
/// [[formula-validation-v2]]) — the baffle envelope diameter at axial
/// distance `s_mm` from the focal plane toward the primary.
///
/// - `field_radius_h_mm`: fully-illuminated field radius `h` at the focal
///   plane (half of `field_diam_mm`).
/// - `aperture_d_mm`: full primary aperture `D`.
/// - `system_focal_length_f_mm`: system focal length `F`.
pub fn baffle_diameter_at(
    s_mm: f64,
    field_radius_h_mm: f64,
    aperture_d_mm: f64,
    system_focal_length_f_mm: f64,
) -> f64 {
    let s_over_f = s_mm / system_focal_length_f_mm;
    2.0 * field_radius_h_mm * (1.0 - s_over_f) + aperture_d_mm * s_over_f
}

/// Compute the Cassegrain-family `BaffleSpec` for a system of full
/// aperture `aperture_d_mm` (`D`), system focal length
/// `system_focal_length_f_mm` (`F`), primary-vertex-to-focal-plane
/// backfocus `backfocus_b_mm` (`b`), and primary-to-secondary mirror
/// separation `mirror_separation_d_mm` (`d`, Schroeder's `d`).
///
/// Returns `None` when `field_diam_mm` is absent (baffle sizing needs a
/// fully-illuminated field size — not applicable/skippable, not an
/// error, matching the graceful-`Option` pattern used by
/// `fabrication::compute`'s other subsystems).
///
/// Evaluates `Ø_baffle(s)` at 3 representative positions
/// ([[v2-baffle-vignetting]] worked example): the primary's central hole
/// (`s = b`), midway between the mirrors (`s = b + d/2`), and the
/// secondary's plane (`s = b + d`).
pub fn compute(
    aperture_d_mm: f64,
    system_focal_length_f_mm: f64,
    backfocus_b_mm: f64,
    mirror_separation_d_mm: f64,
    field_diam_mm: Option<f64>,
) -> Option<BaffleSpec> {
    let field_diam_mm = field_diam_mm?;
    let h = field_diam_mm / 2.0;

    let s_primary = backfocus_b_mm;
    let s_midway = backfocus_b_mm + mirror_separation_d_mm / 2.0;
    let s_secondary = backfocus_b_mm + mirror_separation_d_mm;

    let primary_baffle_hole_diameter_mm =
        baffle_diameter_at(s_primary, h, aperture_d_mm, system_focal_length_f_mm);
    let midway_baffle_diameter_mm =
        baffle_diameter_at(s_midway, h, aperture_d_mm, system_focal_length_f_mm);
    let secondary_baffle_tube_diameter_mm =
        baffle_diameter_at(s_secondary, h, aperture_d_mm, system_focal_length_f_mm);

    let linear_obstruction_pct = 100.0 * secondary_baffle_tube_diameter_mm / aperture_d_mm;
    let fully_illuminated_field_deg =
        2.0 * (h / system_focal_length_f_mm).atan().to_degrees();

    let notes = vec![
        "baffle length is a free mechanical/stray-light-closure parameter, not computed by this tool \
         (secondary_baffle_tube_length_mm is None — see [[v2-baffle-vignetting]] B1 resolution)"
            .to_string(),
    ];

    Some(BaffleSpec {
        kind: "cassegrain".to_string(),
        primary_baffle_hole_diameter_mm,
        midway_baffle_diameter_mm,
        secondary_baffle_tube_diameter_mm,
        secondary_baffle_tube_length_mm: None,
        linear_obstruction_pct,
        fully_illuminated_field_deg,
        vignetting_free: true,
        notes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A2 RC worked example ([[formula-validation-v2]] B1 VERIFIED,
    /// [[v2-baffle-vignetting]] worked-example table): D=200mm, F=1600mm,
    /// b=150mm, d=395.4545mm, field_diam_mm=20mm (2h=20mm).
    const D: f64 = 200.0;
    const F: f64 = 1600.0;
    const B: f64 = 150.0;
    const DSEP: f64 = 395.4545;

    #[test]
    fn worked_example_primary_hole_diameter() {
        let spec = compute(D, F, B, DSEP, Some(20.0)).expect("field_diam_mm present");
        assert!((spec.primary_baffle_hole_diameter_mm - 36.9).abs() < 0.1);
    }

    #[test]
    fn worked_example_midway_diameter() {
        let spec = compute(D, F, B, DSEP, Some(20.0)).expect("field_diam_mm present");
        assert!((spec.midway_baffle_diameter_mm - 59.1).abs() < 0.1);
    }

    #[test]
    fn worked_example_secondary_diameter_and_obstruction() {
        let spec = compute(D, F, B, DSEP, Some(20.0)).expect("field_diam_mm present");
        assert!((spec.secondary_baffle_tube_diameter_mm - 81.4).abs() < 0.1);
        assert!((spec.linear_obstruction_pct - 40.7).abs() < 0.5);
    }

    /// `secondary_baffle_tube_length_mm` must stay `None` per B1's
    /// resolution — never a computed value.
    #[test]
    fn tube_length_is_always_none() {
        let spec = compute(D, F, B, DSEP, Some(20.0)).expect("field_diam_mm present");
        assert_eq!(spec.secondary_baffle_tube_length_mm, None);
    }

    /// `field_diam_mm` absent -> `None`, not an error (graceful
    /// not-applicable pattern).
    #[test]
    fn none_when_field_diam_mm_absent() {
        assert_eq!(compute(D, F, B, DSEP, None), None);
    }

    /// `kind` is always `"cassegrain"` — this chunk's only implemented
    /// variant (Newtonian scoped out, see module doc comment).
    #[test]
    fn kind_is_cassegrain() {
        let spec = compute(D, F, B, DSEP, Some(20.0)).expect("field_diam_mm present");
        assert_eq!(spec.kind, "cassegrain");
    }

    /// `vignetting_free` is always `true` — this tool sizes the baffle TO
    /// the requested field, not the reverse (V-11).
    #[test]
    fn vignetting_free_is_always_true() {
        let spec = compute(D, F, B, DSEP, Some(20.0)).expect("field_diam_mm present");
        assert!(spec.vignetting_free);
    }

    /// `notes` explains the baffle-length limitation.
    #[test]
    fn notes_explain_length_not_computed() {
        let spec = compute(D, F, B, DSEP, Some(20.0)).expect("field_diam_mm present");
        assert!(spec.notes.iter().any(|n| n.contains("free mechanical")));
    }

    /// `baffle_diameter_at` at `s=0` (the focal plane itself) must equal
    /// `2h` exactly — the profile's boundary condition.
    #[test]
    fn baffle_diameter_at_s_zero_equals_2h() {
        let d = baffle_diameter_at(0.0, 10.0, D, F);
        assert!((d - 20.0).abs() < 1e-9);
    }

    /// `baffle_diameter_at` at `s=F` must equal `D` exactly — the profile
    /// converges to the full-aperture virtual disk at one focal length.
    #[test]
    fn baffle_diameter_at_s_equals_f_equals_aperture() {
        let d = baffle_diameter_at(F, 10.0, D, F);
        assert!((d - D).abs() < 1e-9);
    }
}
