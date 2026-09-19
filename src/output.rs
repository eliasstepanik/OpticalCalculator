//! `SpecSheet` output data model (Chunk 8).
//!
//! Renderers (`render_table`/`export_json`/`export_toml`/
//! `write_output_file`, Chunk 14 per architecture.md §4) live in
//! [`crate::render`] — kept in a separate module because this file plus the
//! rendering logic would exceed the 300-line source limit
//! ([[constraints]]/AGENTS.md); the struct-vs-behavior split keeps each file
//! cohesive on its own (data shape here, presentation logic there).
//!
//! Every pipeline (`designs::newtonian`/`cassegrain`/`refractor`) produces
//! one `SpecSheet`, but only populates the sections its `DesignDef.outputs`
//! list names (architecture.md §4 "Output sections are gated by
//! `def.outputs`") — hence the design-specific fields below are all
//! `Option`/`Vec`, not required. `notes` carries the predesign-only /
//! heuristic caveats architecture.md §8 (Q4) and §3 (quintuplet power-split)
//! require surfacing to the user.

/// Diffraction-limited performance: Dawes limit, Airy disk, useful
/// magnification range (`common.rs`, architecture.md §7).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DiffractionSpec {
    pub dawes_limit_arcsec: f64,
    pub airy_disk_diameter_um: f64,
    pub useful_mag_min: f64,
    pub useful_mag_max: f64,
}

/// Field of view: plate scale and true FOV from an eyepiece field stop
/// and/or imaging sensor diagonal (`common.rs`, architecture.md §7).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FovSpec {
    pub plate_scale_arcsec_per_mm: f64,
    pub true_fov_arcmin: Option<f64>,
    pub sensor_fov_arcmin: Option<f64>,
}

/// Newtonian secondary (diagonal) mirror sizing + obstruction, plus the
/// primary mirror's conic constant (`designs/newtonian.rs`, Chunk 11,
/// formula-validation.md A1). `primary_conic` lives here rather than as a
/// new top-level `SpecSheet` field (Chunk 8 retrospect Gap #1) specifically
/// to avoid forcing every other `SpecSheet` constructor (e.g.
/// `designs/cassegrain.rs`, a parallel in-flight chunk) to add a field it
/// has no use for — Newtonian is the only pipeline with a fixed conic
/// override today (architecture.md §2a); RC/classical Cassegrain already
/// carry their own conics on `CassegrainSpec`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NewtonianSecondarySpec {
    pub minor_axis_mm: f64,
    pub offset_mm: f64,
    pub obstruction_linear_pct: f64,
    pub obstruction_areal_pct: f64,
    pub primary_conic: f64,
}

/// RC/classical Cassegrain conics, back-focus, secondary magnification
/// (`designs/cassegrain.rs`, Chunk 12). Radii/separation added in Chunk 12
/// alongside the real formulas (Schroeder *Astronomical Optics* 2nd ed.
/// §6.2) — the sheet needs the physical geometry, not just the conics.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CassegrainSpec {
    pub k1_primary_conic: f64,
    pub k2_secondary_conic: f64,
    pub back_focus_mm: f64,
    pub secondary_magnification: f64,
    /// Primary-to-secondary mirror separation, mm (Schroeder `d`).
    pub mirror_separation_mm: f64,
    /// Primary mirror radius of curvature, mm (concave, `R1 = 2·f1`).
    pub r1_mm: f64,
    /// Secondary mirror radius of curvature, mm (convex).
    pub r2_mm: f64,
}

/// One refractor element's prescription: glass, radii, power
/// (`designs/refractor.rs` + `refractor_chromatic.rs`, Chunks 9/10/13).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RefractorElementSpec {
    pub glass_name: String,
    pub power_mm_inv: f64,
    pub r1_mm: f64,
    pub r2_mm: f64,
}

/// Full N-element chromatic prescription — one entry per element, ordered
/// to match `DesignDef.glasses` (architecture.md §2a/§3).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RefractorPrescriptionSpec {
    pub elements: Vec<RefractorElementSpec>,
}

/// Computed telescope optics spec, generic enough to represent output from
/// any of the 5 (soon 6+, architecture.md §6) design types. Fields beyond
/// the universal basics are `Option`/`Vec` since no single design populates
/// every section — a Newtonian sheet has `newtonian_secondary` but no
/// `refractor_prescription`, and vice versa (architecture.md §4).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SpecSheet {
    /// Design id from the request (`Request::design`), e.g. `"newtonian"`.
    pub design_name: String,
    pub aperture_mm: f64,
    pub focal_length_mm: f64,
    pub f_ratio: f64,

    pub diffraction: Option<DiffractionSpec>,
    pub fov: Option<FovSpec>,
    pub newtonian_secondary: Option<NewtonianSecondarySpec>,
    pub cassegrain: Option<CassegrainSpec>,
    pub refractor_prescription: Option<RefractorPrescriptionSpec>,

    /// Caveats/limitations to print alongside the sheet, e.g. "predesign
    /// only, not fabrication-ready" (architecture.md §8 Q4) or "quintuplet
    /// power-split is a documented heuristic not an optimized design"
    /// (architecture.md §3).
    pub notes: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `SpecSheet` can be built with only the universal basics populated
    /// (e.g. a design whose pipeline hasn't computed optional sections yet)
    /// — every design-specific field must accept `None`/empty without any
    /// required-but-missing compile error.
    #[test]
    fn spec_sheet_builds_with_only_basics_populated() {
        let sheet = SpecSheet {
            design_name: "newtonian".to_string(),
            aperture_mm: 200.0,
            focal_length_mm: 1200.0,
            f_ratio: 6.0,
            diffraction: None,
            fov: None,
            newtonian_secondary: None,
            cassegrain: None,
            refractor_prescription: None,
            notes: vec!["predesign only, not fabrication-ready".to_string()],
        };
        assert_eq!(sheet.design_name, "newtonian");
        assert!(sheet.newtonian_secondary.is_none());
        assert!(sheet.refractor_prescription.is_none());
        assert_eq!(sheet.notes.len(), 1);
    }

    /// Serialization round-trip sanity (Chunk 14 needs `serde::Serialize`
    /// for JSON export) — confirms the derive actually works end-to-end,
    /// not just compiles.
    #[test]
    fn spec_sheet_serializes_to_json() {
        let sheet = SpecSheet {
            design_name: "quintuplet_apo".to_string(),
            aperture_mm: 100.0,
            focal_length_mm: 550.0,
            f_ratio: 5.5,
            diffraction: Some(DiffractionSpec {
                dawes_limit_arcsec: 1.16,
                airy_disk_diameter_um: 7.38,
                useful_mag_min: 16.6,
                useful_mag_max: 200.0,
            }),
            fov: None,
            newtonian_secondary: None,
            cassegrain: None,
            refractor_prescription: Some(RefractorPrescriptionSpec {
                elements: vec![RefractorElementSpec {
                    glass_name: "FPL53".to_string(),
                    power_mm_inv: 0.001,
                    r1_mm: 300.0,
                    r2_mm: -300.0,
                }],
            }),
            notes: vec!["quintuplet power-split is a documented heuristic not an optimized design".to_string()],
        };
        let json = serde_json::to_string(&sheet).expect("SpecSheet must serialize");
        assert!(json.contains("\"design_name\":\"quintuplet_apo\""));
        assert!(json.contains("FPL53"));
    }
}
