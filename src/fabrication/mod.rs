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
//! **Chunk V4 addition:** `substrate_mass` submodule declared below so the
//! new `src/fabrication/substrate_mass.rs` (physics for `SubstrateSpec`,
//! [[v2-substrate-mass]]) is part of the crate tree. This is the minimal
//! module registration only — calling `substrate_mass::compute` from this
//! file's `compute()` dispatch match below is Chunk V9's job per
//! [[task-snapshot-v2-phaseA]]'s chunk table, left untouched here.
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
/// **Phase A (this chunk, V2):** `substrate` (#1+2) and `throughput` (#8)
/// fields exist now; real computation lands in V4/V6.
///
/// **Phase B (later chunks):** `baffle: Option<BaffleSpec>` (#3,
/// [[v2-baffle-vignetting]]), `tolerance: Option<ToleranceSpec>` (#4,
/// [[v2-tolerance-collimation]]), `support: Option<SupportSpec>` (#6,
/// [[v2-support-seidel]]) are NOT yet fields on this struct — they land
/// when their subsystem chunks (V7 tolerance, then the Phase B chunk set)
/// are built, per [[task-snapshot-v2-phaseA]]/[[task-snapshot-v2-phaseB]].
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
    /// "Not applicable to this design" / "not yet computed" / caveat
    /// messages — mirrors `SpecSheet.notes` (`output.rs`).
    pub notes: Vec<String>,
}

/// Resolve which fabrication subsystem keys apply to `def`, per
/// [[architecture-v2]] §2a: an explicit `def.fab_applicable` list wins;
/// absent, infer a default from `def.kind` ("reflector"/"catadioptric" ->
/// substrate/tolerance/support/throughput; "refractor" -> throughput only).
fn resolve_fab_applicable(def: &crate::config::DesignDef) -> Vec<String> {
    match &def.fab_applicable {
        Some(list) => list.clone(),
        None => match def.kind.as_str() {
            "reflector" | "catadioptric" => vec![
                "substrate".to_string(),
                "tolerance".to_string(),
                "support".to_string(),
                "throughput".to_string(),
            ],
            "refractor" => vec!["throughput".to_string()],
            _ => vec![],
        },
    }
}

/// Compute the fabrication report for an already-computed `SpecSheet`
/// ([[architecture-v2]] §1's `fabrication::compute` seam).
///
/// **Chunk V2 scope:** dispatch skeleton only. Checks which subsystems
/// `def` supports (`resolve_fab_applicable`) and records a `notes` entry
/// for each of the two subsystems this struct currently models
/// (`substrate`, `throughput`) — "applicable, not yet computed" if
/// supported, "not applicable" otherwise. Both fields are always `None`
/// this chunk; V4/V6 replace these branches with real computation. Never
/// panics — an unrecognized `def.kind` simply resolves to no applicable
/// subsystems, not an error (graceful-stub pattern, matches v1 Chunk 8).
///
/// `catalogs` argument intentionally omitted: `SubstrateCatalog`/
/// `CoatingCatalog` (V3/V5) don't exist yet. Per the brief's own
/// allowance ("catalogs TBD by V3/V5 signature, use placeholder types or
/// Option args for now"), V4/V6 will add the parameter when it's needed.
pub fn compute(
    sheet: &crate::output::SpecSheet,
    _req: &crate::request::Request,
    def: &crate::config::DesignDef,
) -> Result<FabricationReport, crate::error::OptError> {
    let applicable = resolve_fab_applicable(def);
    let mut notes = Vec::new();

    let substrate = if applicable.iter().any(|s| s == "substrate") {
        notes.push(
            "substrate mass: applicable to this design but not yet computed (pending chunk V4)"
                .to_string(),
        );
        None
    } else {
        notes.push(format!(
            "substrate mass: not applicable to design '{}' (kind='{}')",
            sheet.design_name, def.kind
        ));
        None
    };

    let throughput = if applicable.iter().any(|s| s == "throughput") {
        notes.push(
            "throughput: applicable to this design but not yet computed (pending chunk V6)"
                .to_string(),
        );
        None
    } else {
        notes.push(format!(
            "throughput: not applicable to design '{}' (kind='{}')",
            sheet.design_name, def.kind
        ));
        None
    };

    Ok(FabricationReport {
        design_name: sheet.design_name.clone(),
        substrate,
        throughput,
        notes,
    })
}

#[cfg(test)]
mod tests;
