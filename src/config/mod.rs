//! Config serde structs + loaders for `designs.toml` and `glasses.toml`.
//!
//! Filled in Chunk 5 per architecture.md §2 / §4. The user request TOML
//! (§2c) is `request.rs`'s concern (Chunk 6), not this module's.
//!
//! v2 Chunk V1: split `src/config.rs` into `src/config/mod.rs` (this file,
//! all non-test code, unchanged) + `src/config/tests.rs` (the `#[cfg(test)]
//! mod tests` body, moved verbatim) to stay under the 300-line limit after
//! adding `DesignDef::n_mirrors`/`fab_applicable` — same sibling-file split
//! precedent as `refractor_chromatic.rs`/`refractor_chromatic_solve.rs`.
//! `main.rs`'s `mod config;` declaration is unaffected: Rust resolves
//! `mod config;` to `config/mod.rs` automatically.

use std::collections::HashMap;
use std::path::Path;

use crate::error::OptError;
use crate::glass::{Glass, GlassCatalog};

/// A design's fixed conic override (architecture.md §2a — Newtonian's
/// primary mirror is always a parabola, `conic.primary = -1.0`).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ConicDef {
    pub primary: f64,
}

/// One `[designs.<name>]` entry from `designs.toml` (architecture.md §2a).
///
/// `rc`/`cassegrain` designs share `pipeline = "cassegrain"` and set
/// `conic_mode` to pick between RC and classical formulas; refractor designs
/// set `glasses` (ordered list feeding the chromatic solver) and, for the
/// quintuplet's underdetermined 5-element system, `power_split` (§3, Q1).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct DesignDef {
    /// Dispatch key into `designs::dispatch` (components.md).
    pub pipeline: String,
    /// Element/surface count (primary+secondary mirrors, or lens count).
    pub elements: usize,
    /// `"reflector"` | `"refractor"` | `"catadioptric"`.
    pub kind: String,
    /// Request fields this design requires; `Request::validate` checks these.
    pub required: Vec<String>,
    /// Request fields this design accepts but does not require.
    #[serde(default)]
    pub optional: Vec<String>,
    /// Which `SpecSheet` sections this design populates (output.rs gate).
    pub outputs: Vec<String>,
    /// Fixed conic override, e.g. Newtonian's parabolic primary.
    #[serde(default)]
    pub conic: Option<ConicDef>,
    /// `"rc"` | `"classical"` — selects the Cassegrain formula variant.
    #[serde(default)]
    pub conic_mode: Option<String>,
    /// Ordered glass-catalog names feeding the N-element chromatic solver.
    #[serde(default)]
    pub glasses: Option<Vec<String>>,
    /// Power-split ratios pinning the quintuplet's 2 free DOF (§3, Q1).
    #[serde(default)]
    pub power_split: Option<Vec<f64>>,

    /// v2 (`--fabrication`, #8). Number of reflective surfaces feeding
    /// `system_throughput_pct = (reflectivity/100)^n_mirrors` — Newtonian
    /// and Cassegrain-family designs both use 2, refractors omit this
    /// (or set 0). `None` when unset (v1 configs, or refractor entries
    /// that never list it).
    #[serde(default)]
    pub n_mirrors: Option<u32>,

    /// v2 (`--fabrication`). Fabrication-analogue of `outputs` above: the
    /// subsystem keys (e.g. `"substrate"`, `"tolerance"`, `"throughput"`)
    /// that `fabrication::compute` should attempt for this design. Absent
    /// (`None`) means the fabrication pass infers a default from `kind`.
    #[serde(default)]
    pub fab_applicable: Option<Vec<String>>,
}

/// Top-level `designs.toml` container: `[designs.<name>]` map keyed by
/// design id (e.g. `"newtonian"`, `"rc"`, `"quintuplet_apo"`).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct DesignsFile {
    pub designs: HashMap<String, DesignDef>,
}

/// One `[glasses.<NAME>]` table body from `glasses.toml` (architecture.md
/// §2b). The TOML key supplies `Glass::name`; this struct holds only the
/// three fields the table body actually contains — `name` is never repeated
/// inside the table (see notes/optical-calculator-glasses-toml-wrapper-shape.md).
#[derive(Debug, Clone, serde::Deserialize)]
struct GlassEntry {
    nd: f64,
    vd: f64,
    pgf: f64,
}

/// Top-level `glasses.toml` container: `[glasses.<NAME>]` map keyed by glass
/// name.
#[derive(Debug, Clone, serde::Deserialize)]
struct GlassesFile {
    glasses: HashMap<String, GlassEntry>,
}

/// Read and parse `designs.toml` at `path`.
///
/// Never panics on missing/malformed input: I/O failures map to
/// `OptError::Io`, TOML parse failures to `OptError::Parse`, both carrying
/// `path` for a readable error message.
pub fn load_designs(path: &Path) -> Result<DesignsFile, OptError> {
    let raw = std::fs::read_to_string(path).map_err(|source| OptError::Io {
        path: path.display().to_string(),
        source,
    })?;
    toml::from_str(&raw).map_err(|source| OptError::Parse {
        path: path.display().to_string(),
        source,
    })
}

/// Read, parse, and convert `glasses.toml` at `path` into a `GlassCatalog`.
///
/// Converts each `(name, GlassEntry)` pair into a `glass::Glass` — the map
/// key becomes `Glass::name` since the table body itself has no `name`
/// field.
pub fn load_glasses(path: &Path) -> Result<GlassCatalog, OptError> {
    let raw = std::fs::read_to_string(path).map_err(|source| OptError::Io {
        path: path.display().to_string(),
        source,
    })?;
    let file: GlassesFile = toml::from_str(&raw).map_err(|source| OptError::Parse {
        path: path.display().to_string(),
        source,
    })?;
    let glasses: Vec<Glass> = file
        .glasses
        .into_iter()
        .map(|(name, e)| Glass {
            name,
            nd: e.nd,
            vd: e.vd,
            pgf: e.pgf,
        })
        .collect();
    Ok(GlassCatalog::from_glasses(glasses))
}


#[cfg(test)]
mod tests;
