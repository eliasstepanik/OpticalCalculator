//! Typed error enum for OpticalCalculator.
//!
//! Idiomatic `Result`, no panics on user input (architecture.md §5,
//! constraints.md #5). The library layer never `unwrap`s user data;
//! `main.rs` wraps everything with `anyhow::Context` for a readable exit
//! message.
//!
//! ponytail: variants are not yet constructed anywhere (config.rs, request.rs,
//! designs/, refractor_chromatic.rs land in chunks 3+), so `cargo build` will
//! warn `variant is never constructed` for each one. That's expected during
//! scaffolding — no blanket `#[allow(dead_code)]` here since these ARE real,
//! load-bearing variants, not placeholders. The warnings self-resolve as soon
//! as chunks 3/5/6/9 return `Err(OptError::Variant)`.

/// All error conditions surfaced by the OpticalCalculator library layer.
#[derive(thiserror::Error, Debug)]
pub enum OptError {
    /// A TOML config file failed to parse.
    #[error("config parse error in {path}: {source}")]
    Parse {
        path: String,
        source: toml::de::Error,
    },

    /// `request.design` does not name a key in `designs.toml`.
    #[error("unknown design '{0}' (not in designs.toml)")]
    UnknownDesign(String),

    /// A `DesignDef.pipeline` string does not match any registered pipeline.
    #[error("unknown formula pipeline '{0}'")]
    UnknownPipeline(String),

    /// `Request::validate` found a field in `DesignDef.required` missing
    /// from the user request.
    #[error("design '{design}' requires field '{field}'")]
    MissingField { design: String, field: String },

    /// A glass name referenced by a design or request is not in the catalog.
    #[error("glass '{0}' not found in catalog")]
    UnknownGlass(String),

    /// The chromatic linear system (`solve_linear`) hit a zero/near-zero
    /// pivot — e.g. two glasses with identical Abbe number.
    #[error(
        "chromatic system is singular/ill-conditioned for glasses {0:?} — pick a lower-dispersion element"
    )]
    Unsolvable(Vec<String>),

    /// Filesystem I/O failure while writing an export (`--export`).
    #[error("io error writing {path}: {source}")]
    Io {
        path: String,
        source: std::io::Error,
    },
}
