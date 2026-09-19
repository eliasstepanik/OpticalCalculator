//! `SpecSheet` renderers: stdout table, JSON/TOML export, file writer
//! (Chunk 14 per architecture.md §4). Split from `output.rs` (which holds
//! only the `SpecSheet` data model) to stay under the 300-line source limit
//! ([[constraints]]/AGENTS.md) — this file owns presentation/serialization
//! logic exclusively, `output.rs` owns the struct shapes.

use crate::error::OptError;
use crate::output::SpecSheet;

/// Render a `SpecSheet` as a readable plain-text table for stdout.
///
/// `ponytail:` a `String` buffer built with `writeln!` is enough for a CLI
/// spec sheet — no table-formatting crate needed (architecture.md §4 "stdout
/// spec-sheet table by default").
pub fn render_table(sheet: &SpecSheet) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();

    let _ = writeln!(out, "=== OpticalCalculator Spec Sheet ===");
    let _ = writeln!(out, "Design:          {}", sheet.design_name);
    let _ = writeln!(out, "Aperture:        {:.2} mm", sheet.aperture_mm);
    let _ = writeln!(out, "Focal length:    {:.2} mm", sheet.focal_length_mm);
    let _ = writeln!(out, "f-ratio:         f/{:.2}", sheet.f_ratio);

    if let Some(d) = &sheet.diffraction {
        let _ = writeln!(out, "\n-- Diffraction-limited performance --");
        let _ = writeln!(out, "Dawes limit:     {:.2} arcsec", d.dawes_limit_arcsec);
        let _ = writeln!(out, "Airy disk diam:  {:.2} um", d.airy_disk_diameter_um);
        let _ = writeln!(
            out,
            "Useful mag:      {:.1}x - {:.1}x",
            d.useful_mag_min, d.useful_mag_max
        );
    }

    if let Some(f) = &sheet.fov {
        let _ = writeln!(out, "\n-- Field of view --");
        let _ = writeln!(
            out,
            "Plate scale:     {:.3} arcsec/mm",
            f.plate_scale_arcsec_per_mm
        );
        if let Some(true_fov) = f.true_fov_arcmin {
            let _ = writeln!(out, "True FOV:        {:.2} arcmin", true_fov);
        }
        if let Some(sensor_fov) = f.sensor_fov_arcmin {
            let _ = writeln!(out, "Sensor FOV:      {:.2} arcmin", sensor_fov);
        }
    }

    if let Some(n) = &sheet.newtonian_secondary {
        let _ = writeln!(out, "\n-- Newtonian secondary (diagonal) --");
        let _ = writeln!(out, "Minor axis:      {:.2} mm", n.minor_axis_mm);
        let _ = writeln!(out, "Offset:          {:.4} mm", n.offset_mm);
        let _ = writeln!(
            out,
            "Obstruction:     {:.1}% linear / {:.2}% areal",
            n.obstruction_linear_pct, n.obstruction_areal_pct
        );
        let _ = writeln!(out, "Primary conic:   {:.3}", n.primary_conic);
    }

    if let Some(c) = &sheet.cassegrain {
        let _ = writeln!(out, "\n-- Cassegrain geometry --");
        let _ = writeln!(out, "Primary conic k1:    {:.4}", c.k1_primary_conic);
        let _ = writeln!(out, "Secondary conic k2:  {:.4}", c.k2_secondary_conic);
        let _ = writeln!(out, "Back focus:          {:.2} mm", c.back_focus_mm);
        let _ = writeln!(out, "Secondary mag:       {:.3}x", c.secondary_magnification);
        let _ = writeln!(out, "Mirror separation:   {:.2} mm", c.mirror_separation_mm);
        let _ = writeln!(out, "R1 (primary):        {:.2} mm", c.r1_mm);
        let _ = writeln!(out, "R2 (secondary):      {:.2} mm", c.r2_mm);
    }

    if let Some(p) = &sheet.refractor_prescription {
        let _ = writeln!(out, "\n-- Refractor prescription --");
        let _ = writeln!(
            out,
            "{:<10} {:>14} {:>12} {:>12}",
            "Glass", "Power (1/mm)", "R1 (mm)", "R2 (mm)"
        );
        for (i, el) in p.elements.iter().enumerate() {
            let _ = writeln!(
                out,
                "{:>2}. {:<6} {:>14.6} {:>12.2} {:>12.2}",
                i + 1,
                el.glass_name,
                el.power_mm_inv,
                el.r1_mm,
                el.r2_mm
            );
        }
    }

    if !sheet.notes.is_empty() {
        let _ = writeln!(out, "\n-- Notes --");
        for note in &sheet.notes {
            let _ = writeln!(out, "* {}", note);
        }
    }

    out
}

/// Serialize a `SpecSheet` to pretty-printed JSON.
///
/// `SpecSheet` is a plain data struct — string/f64/Vec/Option fields only, no
/// map keys, no cycles — so `serde_json::to_string_pretty` is provably
/// infallible for this shape (the only serializer error paths are non-string
/// map keys and I/O errors from a writer, neither of which applies to
/// `to_string_pretty` over this struct). `.expect()` documents that rather
/// than threading an unreachable `Result` error path through every caller.
///
/// **Known limitation:** JSON has no literal for infinity/NaN, so
/// non-finite `f64` (e.g. `refractor_shape.rs`'s infinite flat-surface
/// radii) serializes as `null` and won't deserialize back. Use
/// `export_toml` for prescriptions with flat surfaces — TOML writes
/// `inf`/`-inf` literally and round-trips. See KB
/// `notes/rust-serde-json-infinity-null-roundtrip-break.md`.
pub fn export_json(sheet: &SpecSheet) -> Result<String, OptError> {
    Ok(serde_json::to_string_pretty(sheet).expect("SpecSheet is a plain data shape; cannot fail"))
}

/// Serialize a `SpecSheet` to pretty-printed TOML.
///
/// Same infallibility argument as `export_json`: no non-string map keys, no
/// cycles, no `NaN`/`Infinity`-rejecting float validation on this crate's
/// `toml` version for this struct shape.
pub fn export_toml(sheet: &SpecSheet) -> Result<String, OptError> {
    Ok(toml::to_string_pretty(sheet).expect("SpecSheet is a plain data shape; cannot fail"))
}

/// Write a `SpecSheet` to `path`, inferring format from the file extension:
/// `.json` -> JSON, `.toml` -> TOML, anything else -> plain text
/// (`render_table`), per the confirmed `--output <path>` behavior.
pub fn write_output_file(sheet: &SpecSheet, path: &std::path::Path) -> Result<(), OptError> {
    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|e| e.to_ascii_lowercase());
    let contents = match ext.as_deref() {
        Some("json") => export_json(sheet)?,
        Some("toml") => export_toml(sheet)?,
        _ => render_table(sheet),
    };
    std::fs::write(path, contents).map_err(|source| OptError::Io {
        path: path.display().to_string(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::{
        DiffractionSpec, NewtonianSecondarySpec, RefractorElementSpec, RefractorPrescriptionSpec,
    };

    /// Newtonian D=200/f6 worked example (architecture.md §7,
    /// `designs/newtonian.rs::worked_example_matches_formula_validation`):
    /// i=20 -> m=53.00mm, offset=2.2083mm, linear obstruction=26.5%.
    fn sample_newtonian_sheet() -> SpecSheet {
        SpecSheet {
            design_name: "newtonian".to_string(),
            aperture_mm: 200.0,
            focal_length_mm: 1200.0,
            f_ratio: 6.0,
            diffraction: Some(DiffractionSpec {
                dawes_limit_arcsec: 0.58,
                airy_disk_diameter_um: 8.03,
                useful_mag_min: 29.0,
                useful_mag_max: 400.0,
            }),
            fov: None,
            newtonian_secondary: Some(NewtonianSecondarySpec {
                minor_axis_mm: 53.00,
                offset_mm: 2.2083,
                obstruction_linear_pct: 26.5,
                obstruction_areal_pct: 7.0225,
                primary_conic: -1.0,
            }),
            cassegrain: None,
            refractor_prescription: None,
            notes: vec!["predesign only, not fabrication-ready".to_string()],
        }
    }

    /// Flat-surface element (`r2_mm = f64::INFINITY`), as
    /// `refractor_shape.rs` produces for zero-power surfaces.
    fn sample_sheet_with_infinite_radius() -> SpecSheet {
        let mut sheet = sample_newtonian_sheet();
        sheet.newtonian_secondary = None;
        sheet.refractor_prescription = Some(RefractorPrescriptionSpec {
            elements: vec![RefractorElementSpec {
                glass_name: "N-BK7".to_string(),
                power_mm_inv: 0.0,
                r1_mm: 100.0,
                r2_mm: f64::INFINITY,
            }],
        });
        sheet
    }

    #[test]
    fn export_toml_round_trips_infinite_radius() {
        let sheet = sample_sheet_with_infinite_radius();
        let toml_str = export_toml(&sheet).expect("export_toml must succeed");
        let parsed: SpecSheet =
            toml::from_str(&toml_str).expect("TOML with inf radius must round-trip");
        let r2 = parsed.refractor_prescription.unwrap().elements[0].r2_mm;
        assert!(r2.is_infinite() && r2.is_sign_positive(), "r2_mm = {r2}");
    }

    #[test]
    fn export_json_does_not_panic_on_infinite_radius_but_does_not_round_trip() {
        let sheet = sample_sheet_with_infinite_radius();
        let json = export_json(&sheet).expect("export_json must not panic/error on inf");
        assert!(json.contains("\"r2_mm\": null"), "{json}");
        let result: Result<SpecSheet, _> = serde_json::from_str(&json);
        assert!(result.is_err(), "JSON null for inf must NOT round-trip");
    }

    #[test]
    fn render_table_contains_expected_values() {
        let sheet = sample_newtonian_sheet();
        let table = render_table(&sheet);
        assert!(table.contains("newtonian"), "{table}");
        assert!(table.contains("53.00"), "{table}");
        assert!(table.contains("f/6.00"), "{table}");
        assert!(table.contains("predesign only"), "{table}");
    }

    #[test]
    fn export_json_round_trips() {
        let sheet = sample_newtonian_sheet();
        let json = export_json(&sheet).expect("export_json must succeed");
        let parsed: SpecSheet =
            serde_json::from_str(&json).expect("exported JSON must parse back as SpecSheet");
        assert_eq!(parsed.design_name, "newtonian");
        assert!((parsed.newtonian_secondary.unwrap().minor_axis_mm - 53.00).abs() < 1e-9);
    }

    #[test]
    fn export_toml_round_trips() {
        let sheet = sample_newtonian_sheet();
        let toml_str = export_toml(&sheet).expect("export_toml must succeed");
        let parsed: SpecSheet =
            toml::from_str(&toml_str).expect("exported TOML must parse back as SpecSheet");
        assert_eq!(parsed.design_name, "newtonian");
        assert!((parsed.newtonian_secondary.unwrap().minor_axis_mm - 53.00).abs() < 1e-9);
    }

    #[test]
    fn write_output_file_writes_json_to_disk() {
        let sheet = sample_newtonian_sheet();
        let mut path = std::env::temp_dir();
        path.push(format!("optical_calculator_test_{}.json", std::process::id()));

        write_output_file(&sheet, &path).expect("write_output_file must succeed");
        let contents = std::fs::read_to_string(&path).expect("file must exist and be readable");
        assert!(contents.contains("\"design_name\": \"newtonian\""));
        assert!(contents.contains("53.0"));

        std::fs::remove_file(&path).expect("cleanup must succeed");
    }

    #[test]
    fn write_output_file_matches_extension_case_insensitively() {
        let sheet = sample_newtonian_sheet();
        let mut path = std::env::temp_dir();
        path.push(format!(
            "optical_calculator_test_{}_upper.JSON",
            std::process::id()
        ));

        write_output_file(&sheet, &path).expect("write_output_file must succeed");
        let contents = std::fs::read_to_string(&path).expect("file must exist and be readable");
        assert!(
            contents.contains("\"design_name\": \"newtonian\""),
            "uppercase .JSON extension must still produce JSON output: {contents}"
        );

        std::fs::remove_file(&path).expect("cleanup must succeed");
    }

    #[test]
    fn write_output_file_writes_plain_text_for_unknown_extension() {
        let sheet = sample_newtonian_sheet();
        let mut path = std::env::temp_dir();
        path.push(format!("optical_calculator_test_{}.txt", std::process::id()));

        write_output_file(&sheet, &path).expect("write_output_file must succeed");
        let contents = std::fs::read_to_string(&path).expect("file must exist and be readable");
        assert!(contents.contains("=== OpticalCalculator Spec Sheet ==="));

        std::fs::remove_file(&path).expect("cleanup must succeed");
    }
}
