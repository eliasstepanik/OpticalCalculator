//! `DesignPipeline` trait + pipeline-id dispatch (the extensibility seam).
//!
//! Filled in Chunk 8 per components.md / architecture.md §6. This is THE
//! one place a new design type is registered in code (components.md "The
//! dispatch seam") — everything else about a design lives in `designs.toml`.
//! Adding e.g. Maksutov-Cassegrain per architecture.md §6 touches exactly
//! this file's `dispatch` match arm plus one new module; no other module
//! changes.

mod cassegrain;
mod newtonian;
mod refractor;

pub use cassegrain::Cassegrain;
pub use newtonian::Newtonian;
pub use refractor::Refractor;

use crate::config::DesignDef;
use crate::error::OptError;
use crate::glass::GlassCatalog;
use crate::output::SpecSheet;
use crate::request::Request;

/// One design's formula implementation. `compute` receives the validated
/// request, the design's config definition (elements/conic/glasses/outputs
/// gate), and the glass catalog for lookups — architecture.md §4 data flow
/// step "pipeline.compute(req, def, glasses)".
pub trait DesignPipeline {
    fn compute(
        &self,
        req: &Request,
        def: &DesignDef,
        glasses: &GlassCatalog,
    ) -> Result<SpecSheet, OptError>;
}

/// Map a `DesignDef.pipeline` string (architecture.md §2a) to its
/// implementation. `rc`/`cassegrain` designs both set
/// `pipeline = "cassegrain"` and differ only via `DesignDef.conic_mode`,
/// which `Cassegrain::compute` reads (architecture.md §2a) — so there is no
/// separate `"rc"` dispatch arm.
pub fn dispatch(pipeline: &str) -> Result<Box<dyn DesignPipeline>, OptError> {
    Ok(match pipeline {
        "newtonian" => Box::new(Newtonian),
        "cassegrain" => Box::new(Cassegrain), // RC + classical, conic set by def
        "refractor" => Box::new(Refractor),   // N from def.elements
        other => return Err(OptError::UnknownPipeline(other.to_string())),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_known_pipeline_returns_ok() {
        assert!(dispatch("newtonian").is_ok());
        assert!(dispatch("cassegrain").is_ok());
        assert!(dispatch("refractor").is_ok());
    }

    #[test]
    fn dispatch_unknown_pipeline_returns_unknown_pipeline_err() {
        // `Box<dyn DesignPipeline>` isn't `Debug`, so `expect_err` (which
        // requires the `Ok` type to be `Debug`) doesn't apply here — match
        // directly instead.
        match dispatch("bogus") {
            Err(OptError::UnknownPipeline(name)) => assert_eq!(name, "bogus"),
            other => panic!("expected Err(UnknownPipeline), got Ok(..)/other: {}", other.is_ok()),
        }
    }
}
