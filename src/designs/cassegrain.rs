//! RC + classical Cassegrain formulas: conics, back-focus, magnification,
//! obstruction.
//!
//! Real physics filled in Chunk 12 per architecture.md §2a / §4 / §7. The
//! `Cassegrain` type below is a Chunk-8 sequencing seam (same rationale as
//! `newtonian.rs`) — a graceful `Err`, never a panic, so it is safe to be
//! reachable once Chunk 17 wires `main.rs` to the CLI. Chunk 12 replaces
//! the body with real RC/classical conic + back-focus math, reading
//! `def.conic_mode` to pick the variant; it does not need to touch
//! `designs/mod.rs`.

use crate::config::DesignDef;
use crate::designs::DesignPipeline;
use crate::error::OptError;
use crate::glass::GlassCatalog;
use crate::output::SpecSheet;
use crate::request::Request;

/// RC + classical Cassegrain pipeline (`DesignDef.pipeline == "cassegrain"`,
/// variant selected by `DesignDef.conic_mode`).
pub struct Cassegrain;

impl DesignPipeline for Cassegrain {
    fn compute(&self, _req: &Request, _def: &DesignDef, _glasses: &GlassCatalog) -> Result<SpecSheet, OptError> {
        // Chunk 12 fills in real RC/classical conic + back-focus physics here.
        Err(OptError::Unsolvable(vec![
            "cassegrain pipeline not yet implemented — Chunk 12".to_string(),
        ]))
    }
}
