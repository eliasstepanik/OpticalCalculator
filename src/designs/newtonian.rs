//! Newtonian mirror formulas: secondary sizing, illumination, conic (−1.0).
//!
//! Real physics filled in Chunk 11 per architecture.md §4 / §7. The
//! `Newtonian` type below is a Chunk-8 sequencing seam (components.md "The
//! dispatch seam" needs a concrete type to register in `designs::dispatch`
//! before Chunk 11 exists) — it returns a graceful `Err`, never panics, so
//! it is safe to be reachable once Chunk 17 wires `main.rs` to the CLI.
//! Chunk 11 replaces the body with real secondary-sizing/illumination math;
//! it does not need to touch `designs/mod.rs`.

use crate::config::DesignDef;
use crate::designs::DesignPipeline;
use crate::error::OptError;
use crate::glass::GlassCatalog;
use crate::output::SpecSheet;
use crate::request::Request;

/// Newtonian pipeline (`DesignDef.pipeline == "newtonian"`).
pub struct Newtonian;

impl DesignPipeline for Newtonian {
    fn compute(&self, _req: &Request, _def: &DesignDef, _glasses: &GlassCatalog) -> Result<SpecSheet, OptError> {
        // Chunk 11 fills in real secondary-sizing/illumination physics here.
        Err(OptError::Unsolvable(vec![
            "newtonian pipeline not yet implemented — Chunk 11".to_string(),
        ]))
    }
}
