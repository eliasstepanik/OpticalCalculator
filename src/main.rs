//! `optical-calculator` — CLI entry point.
//!
//! Wires every module built in chunks 1-16 per architecture.md §4's data
//! flow: parse args -> load designs/glasses/request -> validate -> dispatch
//! -> compute -> render (+ optional file export). Errors propagate via
//! `anyhow::Context` for a readable stderr message and non-zero exit; the
//! library layer below never panics on user input (architecture.md §5).

mod common;
mod config;
mod designs;
#[cfg(test)]
mod examples_validate;
mod error;
mod fabrication;
mod glass;
mod output;
mod refractor_chromatic;
mod refractor_chromatic_solve;
mod render;
mod request;
mod request_validate;

use std::path::PathBuf;

use anyhow::Context;
use clap::Parser;

use crate::error::OptError;
use crate::request::Request;

/// Compute analytical telescope optics specs from a TOML request file
/// (architecture.md §4).
#[derive(Parser, Debug)]
#[command(version, about)]
struct Cli {
    /// Path to the user request TOML (architecture.md §2c).
    request: PathBuf,

    /// Path to the design-type definitions TOML (architecture.md §2a).
    #[arg(long, default_value = "configs/designs.toml")]
    designs: PathBuf,

    /// Path to the glass catalog TOML (architecture.md §2b).
    #[arg(long, default_value = "configs/glasses.toml")]
    glasses: PathBuf,

    /// Optional output file; format inferred from extension
    /// (`.json`/`.toml`/plain text — see `render::write_output_file`).
    #[arg(long)]
    output: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let designs_file = config::load_designs(&cli.designs)
        .with_context(|| format!("loading designs file {}", cli.designs.display()))?;
    let glass_catalog = config::load_glasses(&cli.glasses)
        .with_context(|| format!("loading glasses file {}", cli.glasses.display()))?;

    let raw_request = std::fs::read_to_string(&cli.request)
        .with_context(|| format!("reading request file {}", cli.request.display()))?;
    let request: Request = toml::from_str(&raw_request)
        .map_err(|source| OptError::Parse {
            path: cli.request.display().to_string(),
            source,
        })
        .with_context(|| format!("parsing request file {}", cli.request.display()))?;

    let design_def = designs_file
        .designs
        .get(&request.design)
        .ok_or_else(|| OptError::UnknownDesign(request.design.clone()))?;

    request.validate(design_def).context("validating request")?;

    let pipeline = designs::dispatch(&design_def.pipeline).context("dispatching design pipeline")?;
    let spec_sheet = pipeline
        .compute(&request, design_def, &glass_catalog)
        .context("computing optical spec sheet")?;

    println!("{}", render::render_table(&spec_sheet));

    if let Some(output_path) = &cli.output {
        render::write_output_file(&spec_sheet, output_path)
            .with_context(|| format!("writing output file {}", output_path.display()))?;
    }

    Ok(())
}
