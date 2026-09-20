//! `FabricationReport` output struct + `fabrication::compute` dispatch
//! (Chunk V2, [[task-snapshot-v2-phaseA]]) — the architectural glue for the
//! opt-in `--fabrication` pass, parallel to v1's Chunk 8 `SpecSheet` seam
//! ([[architecture]] §4, output.rs).
//!
//! [[architecture-v2]] §1 (decision #1): fabrication runs as ONE separate
//! pass, invoked only when the CLI is given `--fabrication` (wired in a
//! later chunk, V9 — NOT here). §4 (decision #4): a DISTINCT
//! `FabricationReport` struct, not more `SpecSheet` fields — this file is
//! that struct plus the dispatch free-function.
//!
//! **Naming correction vs the delegation brief:** the brief's paraphrase
//! named the fields/types `substrate_mass: Option<SubstrateMassSpec>`. The
//! brief itself says "Expected shape (verify against KB doc)" — verifying
//! against [[architecture-v2]] §4's exact `FabricationReport` shape and
//! [[v2-substrate-mass]]'s `### SubstrateSpec (report struct)` heading shows
//! the real names are `substrate: Option<SubstrateSpec>` — used here
//! verbatim per the "same names" architecture rule.
//!
//! **Chunk V2 scope:** only the struct/dispatch skeleton. NO physics —
//! `substrate`/`throughput` fields exist but are always `None` this chunk;
//! V4 ([[v2-substrate-mass]]) and V6 ([[v2-coatings-throughput]]) fill them
//! in. `compute()` never panics on a design it can't yet handle — it notes
//! "not yet computed" or "not applicable", mirroring v1's Chunk 8
//! graceful-stub precedent (`output.rs`'s `Option` fields before their
//! pipelines existed).
//!
//! **Chunk V9 update:** `compute()` below is now the real dispatch —
//! `substrate_mass::compute`/`throughput::compute`/`tolerance::compute_phase_a`
//! are all called and their results populate `FabricationReport`'s
//! `substrate`/`throughput`/`tolerance` fields. See `compute()`'s own doc
//! comment for the per-subsystem gating rules.
//!
//! **V-8 update (Phase B dispatch wiring):** `compute()` now also calls
//! `baffle::compute`/`support::compute`/`tolerance::compute_phase_b_conic`
//! when `"baffle"`/`"support"` are in `fab_applicable` and the design's
//! `SpecSheet` has the geometry each needs. The dispatch logic itself
//! lives in `fabrication/dispatch.rs` (split out to keep this file under
//! the 300-line limit once the Phase B wiring was added) — this file now
//! owns only the module declarations and the report/subsystem struct
//! shapes.
pub mod substrate_mass;

/// **Chunk V6 addition:** `throughput` submodule declared below, same
/// minimal-registration-only pattern as Chunk V4's `substrate_mass` line
/// above — `src/fabrication/throughput.rs` (physics for `ThroughputSpec`,
/// [[v2-coatings-throughput]]) is now part of the crate tree. Calling
/// `throughput::compute` from this file's `compute()` dispatch match below
/// is still Chunk V9's job, left untouched here.
pub mod throughput;

/// **Chunk V7 addition:** `tolerance` submodule declared below, same
/// minimal-registration-only pattern as Chunk V4's `substrate_mass` and
/// Chunk V6's `throughput` lines above — `src/fabrication/tolerance.rs`
/// (Phase-A λ/8-only physics for `ToleranceSpec`,
/// [[v2-tolerance-collimation]]) is now part of the crate tree. Calling
/// `tolerance::compute_phase_a` from this file's `compute()` dispatch
/// match, and adding a `tolerance` field to `FabricationReport`, are still
/// Chunk V9's job, left untouched here.
pub mod tolerance;

/// **Chunk VB1 addition, wired V-8:** `src/fabrication/baffle.rs`
/// (Cassegrain-family physics for `BaffleSpec`, [[v2-baffle-vignetting]]
/// B1 VERIFIED). `baffle::compute` is now called from `dispatch.rs`'s
/// `compute()` when `"baffle"` is in a Cassegrain-family design's
/// `fab_applicable` list, and `FabricationReport.baffle` carries the
/// result.
pub mod baffle;

/// **Chunk VB3 addition, wired V-8:** `src/fabrication/support.rs`
/// (mirror support-sag physics for `SupportSpec`, [[v2-support-seidel]]
/// S1 VERIFIED). `support::compute` is now called from `dispatch.rs`'s
/// `compute()` when `"support"` is in a design's `fab_applicable` list,
/// and `FabricationReport.support` carries the result.
pub mod support;

/// **Chunk VC1 addition:** `src/fabrication/collimation.rs`
/// (two-mirror decenter/tilt-to-coma physics for `CollimationSpec`,
/// [[v2-tolerance-collimation]] C1 VERIFIED). Same
/// "registration only, dispatch deferred" pattern as Chunks
/// V4/V6/V7/VB1/VB3 above: `collimation::compute` is NOT yet called from
/// `dispatch.rs`'s `compute()` match, and `FabricationReport` does not
/// yet carry a `collimation` field — both are left for a later
/// integration chunk (this chunk's brief restricts edits to
/// `collimation.rs` only).
pub mod collimation;

/// **Chunk VC2 addition:** `src/fabrication/seidel.rs` (+ sibling
/// `seidel/seidel_trace.rs` for the paraxial trace machinery,
/// `seidel/tests.rs` for its tests) — two-mirror third-order Seidel
/// aberration sums for `SeidelSpec`, [[v2-support-seidel]] S2 VERIFIED.
/// Same "registration only, dispatch deferred" pattern as Chunk VC1's
/// `collimation` line above: `seidel::compute` is NOT yet called from
/// `dispatch.rs`'s `compute()` match, and `FabricationReport` does not
/// yet carry a `seidel` field — both left for a later integration chunk
/// (this chunk's brief restricts edits to `seidel.rs` + its sibling split
/// files only).
pub mod seidel;

/// V-8 addition: the `compute()` dispatch free-function + its helper
/// functions, split into a sibling file to keep this one under the
/// 300-line limit (this file already carries 5 struct definitions plus
/// module declarations before dispatch logic is even added).
mod dispatch;
pub use dispatch::compute;

/// Mirror-blank substrate mass/thickness result (#1+#2,
/// [[v2-substrate-mass]] `### SubstrateSpec`). Populated by
/// `fabrication::substrate_mass` (Chunk V4) — this chunk only defines the
/// shape; `fabrication::compute` never constructs one yet.
///
/// ponytail: no field is read anywhere yet (V4 hasn't landed), so
/// `cargo build` will warn `struct is never constructed`. Same "expected
/// during scaffolding" pattern as `error.rs`'s v2 variant comment.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SubstrateSpec {
    pub material: String,
    pub thickness_mm: f64,
    /// D / t actually used (may differ from `Request.thickness_ratio` if
    /// unset — see [[v2-substrate-mass]] default D/6).
    pub thickness_ratio: f64,
    pub solid_volume_cm3: f64,
    /// After lightweighting (`solid_volume_cm3` when `lightweight_fraction`
    /// is 0.0).
    pub blank_volume_cm3: f64,
    pub mass_kg: f64,
    /// Plain solid-disk mass, reported alongside `mass_kg` so the user sees
    /// the lightweighting saving.
    pub solid_mass_kg: f64,
    pub lightweight_fraction: f64,
    /// Echoed from the substrate catalog for material comparison.
    pub cte_ppm_per_k: f64,
    pub over_max_aperture: bool,
}

/// System reflective throughput result (#8, [[v2-coatings-throughput]]
/// `### ThroughputSpec`). Populated by `fabrication::throughput` (Chunk
/// V6) — this chunk only defines the shape.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ThroughputSpec {
    pub coating: String,
    pub reflectivity_pct: f64,
    pub n_mirrors: u32,
    /// `100 * reflectivity^n_mirrors`.
    pub system_throughput_pct: f64,
    /// `false` for refractors (`n_mirrors == 0`).
    pub applicable: bool,
}

/// Fabrication/mechanical/optical-quality report for one computed design,
/// derived one-directionally from its `SpecSheet` ([[architecture-v2]] §4).
/// Every subsystem field is `Option` — a design only gets the subsystems
/// its `fab_applicable`/`kind` supports; skipped ones are recorded in
/// `notes`, never an error ([[architecture-v2]] §0 "never errors on N/A").
///
/// **Phase A (V2/V4/V6/V7/V9):** `substrate` (#1+2), `throughput` (#8), and
/// `tolerance` (#4, λ/8-only subset, [[v2-tolerance-collimation]]) are all
/// wired and computed by `compute()` below (Chunk V9,
/// [[task-snapshot-v2-phaseA]] V9 row).
///
/// **Phase B, wired V-8:** `baffle: Option<baffle::BaffleSpec>` (#3,
/// [[v2-baffle-vignetting]]), `support: Option<support::SupportSpec>` (#6,
/// [[v2-support-seidel]]), and `tolerance`'s conic-tolerance fields (T1,
/// via `tolerance::compute_phase_b_conic`) are now computed by `compute()`
/// (`dispatch.rs`) per [[task-snapshot-v2-phaseB]]'s VB4 row.
///
/// **Phase C (BLOCKED, later still):** `collimation: Option<CollimationSpec>`
/// (#5) and `seidel: Option<SeidelSpec>` (#7) are gated behind
/// [[formula-validation-v2]] items C1/S2 ([[architecture-v2]] R1/R2, both
/// HIGH risk) — literature-verified worked examples must exist before
/// either field or its computing module is built.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FabricationReport {
    pub design_name: String,
    pub substrate: Option<SubstrateSpec>,
    pub throughput: Option<ThroughputSpec>,
    /// #4, [[v2-tolerance-collimation]]. Phase A (λ/8-only) added Chunk
    /// V9; Phase B conic-tolerance fields (T1 VERIFIED) populated by
    /// `dispatch.rs`'s `compute()` since V-8, via
    /// `tolerance::compute_phase_b_conic` for Cassegrain-family designs
    /// with conic geometry (falls back to `compute_phase_a` otherwise).
    pub tolerance: Option<crate::fabrication::tolerance::ToleranceSpec>,
    /// #3, [[v2-baffle-vignetting]] (B1 VERIFIED). V-8 addition —
    /// Cassegrain-family designs only ([[v2-baffle-vignetting]] scopes the
    /// Newtonian variant out); `None` for non-Cassegrain designs or when
    /// `field_diam_mm` is absent from the request.
    pub baffle: Option<crate::fabrication::baffle::BaffleSpec>,
    /// #6, [[v2-support-seidel]] (S1 VERIFIED). V-8 addition — any mirror
    /// design (Newtonian and Cassegrain-family alike; a lens has no
    /// support-sag concept).
    pub support: Option<crate::fabrication::support::SupportSpec>,
    /// "Not applicable to this design" / "not yet computed" / caveat
    /// messages — mirrors `SpecSheet.notes` (`output.rs`).
    pub notes: Vec<String>,
}

#[cfg(test)]
mod tests;
