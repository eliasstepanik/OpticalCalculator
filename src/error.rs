//! Typed error enum for OpticalCalculator.
//!
//! Variants (`Parse`, `UnknownDesign`, `UnknownPipeline`, `MissingField`,
//! `UnknownGlass`, `Unsolvable`, `Io`) are filled in Chunk 2 per
//! architecture.md §5.

/// Placeholder error type — variants added in Chunk 2.
#[allow(dead_code)]
#[derive(thiserror::Error, Debug)]
pub enum OptError {}
