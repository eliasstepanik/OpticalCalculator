//! `optical-calculator` — CLI entry point.
//!
//! Wires every module built in chunks 1-16 per architecture.md §4's data
//! flow: parse args -> load designs/glasses/request -> validate -> dispatch
//! -> compute -> render (+ optional file export). Errors propagate via
//! `anyhow::Context` for a readable stderr message and non-zero exit; the
//! library layer below never panics on user input (architecture.md §5).

mod coating;
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
mod render_fabrication;
mod request;
mod request_validate;
mod substrate;

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
    /// Writes the v1 `SpecSheet` to this exact path unchanged
    /// ([[architecture-v2]] §3 data flow: "SpecSheet export unchanged").
    /// When `--fabrication` is also set, the `FabricationReport` is
    /// ADDITIONALLY written next to it via the same extension-inference
    /// logic (`render_fabrication::write_output_file`), at a derived path
    /// with `.fabrication` inserted before the extension (`foo.json` ->
    /// `foo.fabrication.json`; `foo` -> `foo.fabrication`) — "report export
    /// via same extension logic" in the same architecture line.
    #[arg(long)]
    output: Option<PathBuf>,

    /// v2: run the opt-in fabrication/mechanical/optical-quality pass
    /// after the v1 spec sheet ([[architecture-v2]] §1 decision #1).
    /// Without this flag, CLI behavior is byte-identical to v1.
    #[arg(long)]
    fabrication: bool,

    /// Path to the mirror-blank substrate catalog TOML (v2, only loaded
    /// when `--fabrication` is set; [[architecture-v2]] §2b).
    #[arg(long, default_value = "configs/substrates.toml")]
    substrates: PathBuf,

    /// Path to the mirror coating catalog TOML (v2, only loaded when
    /// `--fabrication` is set; [[architecture-v2]] §2c).
    #[arg(long, default_value = "configs/coatings.toml")]
    coatings: PathBuf,
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

    if cli.fabrication {
        let substrate_catalog = config::load_substrates(&cli.substrates)
            .with_context(|| format!("loading substrates file {}", cli.substrates.display()))?;
        let coating_catalog = config::load_coatings(&cli.coatings)
            .with_context(|| format!("loading coatings file {}", cli.coatings.display()))?;

        let report = fabrication::compute(
            &spec_sheet,
            &request,
            design_def,
            &substrate_catalog,
            &coating_catalog,
        )
        .context("computing fabrication report")?;

        println!("{}", render_fabrication::render_table(&report));

        if let Some(output_path) = &cli.output {
            let fabrication_path = fabrication_output_path(output_path);
            render_fabrication::write_output_file(&report, &fabrication_path).with_context(
                || format!("writing fabrication output file {}", fabrication_path.display()),
            )?;
        }
    }

    Ok(())
}

/// Derive the fabrication-report sibling path for `--output`: insert
/// `.fabrication` before the file extension (`foo.json` ->
/// `foo.fabrication.json`), or append it when there is no extension
/// (`foo` -> `foo.fabrication`). Keeps the v1 `SpecSheet` export at the
/// exact `--output` path untouched while giving the fabrication report its
/// own file via the same extension-inference logic
/// ([[architecture-v2]] §3).
fn fabrication_output_path(output_path: &std::path::Path) -> PathBuf {
    match output_path.extension().and_then(|e| e.to_str()) {
        Some(ext) => output_path.with_extension(format!("fabrication.{ext}")),
        None => {
            let mut path = output_path.as_os_str().to_owned();
            path.push(".fabrication");
            PathBuf::from(path)
        }
    }
}
