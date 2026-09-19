//! N-element refractor pipeline: element powers → radii, prescription.
//!
//! Calls the chromatic solver in `refractor_chromatic.rs`. Real physics
//! filled in Chunk 13 per architecture.md §3. The `Refractor` type below is
//! a Chunk-8 sequencing seam (same rationale as `newtonian.rs`) — a graceful
//! `Err`, never a panic, so it is safe to be reachable once Chunk 17 wires
//! `main.rs` to the CLI. Chunk 13 replaces the body with the real
//! `solve_powers` → radii → `RefractorPrescriptionSpec` pipeline; it does
//! not need to touch `designs/mod.rs`.

use crate::config::DesignDef;
use crate::designs::DesignPipeline;
use crate::error::OptError;
use crate::glass::GlassCatalog;
use crate::output::SpecSheet;
use crate::request::Request;

/// N-element refractor pipeline (`DesignDef.pipeline == "refractor"`, N from
/// `DesignDef.elements` — doublet/triplet/quintuplet all share this type).
pub struct Refractor;

impl DesignPipeline for Refractor {
    fn compute(&self, _req: &Request, _def: &DesignDef, _glasses: &GlassCatalog) -> Result<SpecSheet, OptError> {
        // Chunk 13 fills in the real solve_powers -> radii -> prescription
        // pipeline here (calls refractor_chromatic.rs, Chunks 9/10).
        Err(OptError::Unsolvable(vec![
            "refractor pipeline not yet implemented — Chunk 13".to_string(),
        ]))
    }
}
