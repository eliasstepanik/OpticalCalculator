//! `FabricationReport` renderers: stdout table, JSON/TOML export, file
//! writer (Chunk V8 per [[task-snapshot-v2-phaseA]]) — mirrors v1's
//! `render.rs` pattern exactly for the distinct `FabricationReport` output
//! type ([[architecture-v2]] §4: "gets its own renderer
//! (`render_fabrication.rs`) appended after the spec sheet").
//!
//! **Chunk V8 scope:** `FabricationReport` today has exactly two subsystem
//! fields, `substrate` and `throughput` (`fabrication::mod.rs`) — `tolerance`
//! is not yet a field on the struct (that's V9's job per
//! [[task-snapshot-v2-phaseA]]). This file renders only what exists; each
//! section is one `if let Some(...) = &report.field` block so a future
//! field (once added) slots in as one more block without restructuring.

use crate::error::OptError;
use crate::fabrication::FabricationReport;

/// Render a `FabricationReport` as a readable plain-text table for stdout.
///
/// `ponytail:` same `String` + `writeln!` buffer as v1's `render::render_table`
/// — no table-formatting crate needed here either.
pub fn render_table(report: &FabricationReport) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();

    let _ = writeln!(out, "=== OpticalCalculator Fabrication Report ===");
    let _ = writeln!(out, "Design:          {}", report.design_name);

    if let Some(s) = &report.substrate {
        let _ = writeln!(out, "\n-- Substrate / Mirror Blank --");
        let _ = writeln!(out, "Material:        {}", s.material);
        let _ = writeln!(out, "Thickness:       {:.2} mm", s.thickness_mm);
        let _ = writeln!(out, "Blank volume:    {:.2} cm^3", s.blank_volume_cm3);
        let _ = writeln!(out, "Mass:            {:.3} kg", s.mass_kg);
    }

    if let Some(t) = &report.throughput {
        let _ = writeln!(out, "\n-- Optical Throughput --");
        let _ = writeln!(out, "Coating:         {}", t.coating);
        let _ = writeln!(out, "Reflectivity:    {:.2}%", t.reflectivity_pct);
        let _ = writeln!(out, "Mirror count:    {}", t.n_mirrors);
        let _ = writeln!(
            out,
            "System throughput: {:.2}%",
            t.system_throughput_pct
        );
    }

    if let Some(tol) = &report.tolerance {
        let _ = writeln!(out, "\n-- Fabrication Tolerance (Phase A: surface only) --");
        let _ = writeln!(out, "Wavelength:      {:.1} nm", tol.wavelength_nm);
        let _ = writeln!(
            out,
            "Surface P-V tol: {:.2} nm (lambda/{:.0})",
            tol.surface_pv_tol_nm,
            1.0 / tol.surface_pv_tol_waves
        );
        let _ = writeln!(
            out,
            "Wavefront P-V tol: {:.2} nm (lambda/{:.0})",
            tol.wavefront_pv_tol_nm,
            1.0 / tol.wavefront_pv_tol_waves
        );
    }

    if !report.notes.is_empty() {
        let _ = writeln!(out, "\n-- Notes --");
        for note in &report.notes {
            let _ = writeln!(out, "* {}", note);
        }
    }

    out
}

/// Serialize a `FabricationReport` to pretty-printed JSON.
///
/// Same infallibility argument as v1's `render::export_json`:
/// `FabricationReport` is a plain data struct (string/f64/u32/bool/Vec/Option
/// fields only, no map keys, no cycles), so `serde_json::to_string_pretty`
/// cannot fail for this shape.
///
/// **Known limitation:** JSON has no literal for infinity/NaN, so a
/// non-finite `f64` field would serialize as `null` and won't deserialize
/// back — the exact v1 `render::export_json` limitation
/// ([[rust-serde-json-infinity-null-roundtrip-break]]). None of today's
/// Phase A fields (`thickness_mm`, `mass_kg`, `reflectivity_pct`,
/// `surface_pv_tol_nm`, etc.) are constructed as non-finite by their
/// producing subsystems, so this does not currently bite — resolution of
/// follow-up V-6 ([[task-snapshot-v2-phaseA]] session-state.md): documented
/// here rather than code-fixed, since there is no reachable non-finite
/// input in Phase A to guard against yet.
pub fn export_json(report: &FabricationReport) -> Result<String, OptError> {
    Ok(serde_json::to_string_pretty(report)
        .expect("FabricationReport is a plain data shape; cannot fail"))
}

/// Serialize a `FabricationReport` to pretty-printed TOML.
///
/// Same infallibility argument as `export_json`.
pub fn export_toml(report: &FabricationReport) -> Result<String, OptError> {
    Ok(toml::to_string_pretty(report).expect("FabricationReport is a plain data shape; cannot fail"))
}

/// Write a `FabricationReport` to `path`, inferring format from the file
/// extension: `.json` -> JSON, `.toml` -> TOML, anything else -> plain text
/// (`render_table`). Mirrors v1's `render::write_output_file` exactly,
/// including the `.to_ascii_lowercase()` case-insensitivity fix.
pub fn write_output_file(report: &FabricationReport, path: &std::path::Path) -> Result<(), OptError> {
    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|e| e.to_ascii_lowercase());
    let contents = match ext.as_deref() {
        Some("json") => export_json(report)?,
        Some("toml") => export_toml(report)?,
        _ => render_table(report),
    };
    std::fs::write(path, contents).map_err(|source| OptError::Io {
        path: path.display().to_string(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fabrication::{SubstrateSpec, ThroughputSpec};

    /// D=300mm Pyrex, D/6 thickness rule (worked example,
    /// [[task-snapshot-v2-phaseA]] V4 acceptance: t=50.0mm,
    /// V_solid=3534.29cm^3, mass=7.882kg), lightweighted to 0.3 -> 5.517kg.
    fn sample_substrate() -> SubstrateSpec {
        SubstrateSpec {
            material: "Pyrex".to_string(),
            thickness_mm: 50.0,
            thickness_ratio: 6.0,
            solid_volume_cm3: 3534.29,
            blank_volume_cm3: 2473.98,
            mass_kg: 5.517,
            solid_mass_kg: 7.882,
            lightweight_fraction: 0.3,
            cte_ppm_per_k: 3.3,
            over_max_aperture: false,
        }
    }

    /// N=2 enhanced-aluminum throughput (worked example,
    /// [[task-snapshot-v2-phaseA]] V6 acceptance: 94.0% -> 88.36%).
    fn sample_throughput() -> ThroughputSpec {
        ThroughputSpec {
            coating: "enhanced_aluminum".to_string(),
            reflectivity_pct: 94.0,
            n_mirrors: 2,
            system_throughput_pct: 88.36,
            applicable: true,
        }
    }

    fn sample_report() -> FabricationReport {
        FabricationReport {
            design_name: "rc".to_string(),
            substrate: Some(sample_substrate()),
            throughput: Some(sample_throughput()),
            tolerance: None,
            notes: vec!["tolerance: pending chunk V9".to_string()],
        }
    }

    #[test]
    fn render_table_contains_substrate_and_throughput_sections() {
        let report = sample_report();
        let table = render_table(&report);
        assert!(table.contains("Pyrex"), "{table}");
        assert!(table.contains("50.00 mm"), "{table}");
        assert!(table.contains("5.517 kg"), "{table}");
        assert!(table.contains("enhanced_aluminum"), "{table}");
        assert!(table.contains("88.36%"), "{table}");
        assert!(table.contains("tolerance: pending chunk V9"), "{table}");
    }

    #[test]
    fn export_json_round_trips() {
        let report = sample_report();
        let json = export_json(&report).expect("export_json must succeed");
        let parsed: FabricationReport =
            serde_json::from_str(&json).expect("exported JSON must parse back");
        assert_eq!(parsed.design_name, "rc");
        assert!((parsed.substrate.unwrap().mass_kg - 5.517).abs() < 1e-9);
    }

    #[test]
    fn export_toml_round_trips() {
        let report = sample_report();
        let toml_str = export_toml(&report).expect("export_toml must succeed");
        let parsed: FabricationReport =
            toml::from_str(&toml_str).expect("exported TOML must parse back");
        assert_eq!(parsed.design_name, "rc");
        assert!((parsed.throughput.unwrap().system_throughput_pct - 88.36).abs() < 1e-9);
    }

    #[test]
    fn write_output_file_matches_extension_case_insensitively() {
        let report = sample_report();

        let mut lower_path = std::env::temp_dir();
        lower_path.push(format!(
            "optical_calculator_fab_test_{}.json",
            std::process::id()
        ));
        write_output_file(&report, &lower_path).expect("lowercase .json must write");
        let lower_contents =
            std::fs::read_to_string(&lower_path).expect("file must exist and be readable");
        assert!(lower_contents.contains("\"design_name\": \"rc\""));
        std::fs::remove_file(&lower_path).expect("cleanup must succeed");

        let mut upper_path = std::env::temp_dir();
        upper_path.push(format!(
            "optical_calculator_fab_test_{}_upper.JSON",
            std::process::id()
        ));
        write_output_file(&report, &upper_path).expect("uppercase .JSON must write");
        let upper_contents =
            std::fs::read_to_string(&upper_path).expect("file must exist and be readable");
        assert!(
            upper_contents.contains("\"design_name\": \"rc\""),
            "uppercase .JSON extension must still produce JSON output: {upper_contents}"
        );
        std::fs::remove_file(&upper_path).expect("cleanup must succeed");
    }

    #[test]
    fn write_output_file_writes_plain_text_for_unknown_extension() {
        let report = sample_report();
        let mut path = std::env::temp_dir();
        path.push(format!(
            "optical_calculator_fab_test_{}.txt",
            std::process::id()
        ));

        write_output_file(&report, &path).expect("write_output_file must succeed");
        let contents = std::fs::read_to_string(&path).expect("file must exist and be readable");
        assert!(contents.contains("=== OpticalCalculator Fabrication Report ==="));

        std::fs::remove_file(&path).expect("cleanup must succeed");
    }
}
