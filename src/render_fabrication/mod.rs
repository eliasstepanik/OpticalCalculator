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
//!
//! **V-8 update:** `baffle`/`support` sections added, following the same
//! one-`if let`-block-per-field pattern; `tolerance`'s section now also
//! prints the conic-tolerance fields when present (T1 VERIFIED). Split
//! into a directory module (`mod.rs` + `tests.rs`) at this point to stay
//! under the 300-line limit, same sibling-file split precedent as
//! `config/`/`request/`/`fabrication/tolerance/`.

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
        if t.applicable {
            let _ = writeln!(out, "Coating:         {}", t.coating);
            let _ = writeln!(out, "Reflectivity:    {:.2}%", t.reflectivity_pct);
            let _ = writeln!(out, "Mirror count:    {}", t.n_mirrors);
            let _ = writeln!(
                out,
                "System throughput: {:.2}%",
                t.system_throughput_pct
            );
        } else {
            let _ = writeln!(
                out,
                "Optical Throughput: not modeled for refractor designs \
                 (mirror-coating formula only; glass transmission losses \
                 not calculated)"
            );
        }
    }

    if let Some(tol) = &report.tolerance {
        let _ = writeln!(out, "\n-- Fabrication Tolerance --");
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
        if tol.primary_conic_tolerance > 0.0 {
            let _ = writeln!(
                out,
                "Primary conic tol (dK): {:.5}",
                tol.primary_conic_tolerance
            );
            if let Some(frac) = tol.primary_conic_tolerance_fractional {
                let _ = writeln!(out, "  ({:.3}% of |K1|)", frac * 100.0);
            }
        }
        if let Some(sec_tol) = tol.secondary_conic_tolerance {
            let _ = writeln!(out, "Secondary conic tol (dK): {:.5}", sec_tol);
            if let Some(frac) = tol.secondary_conic_tolerance_fractional {
                let _ = writeln!(out, "  ({:.3}% of |K2|)", frac * 100.0);
            }
        }
    }

    if let Some(baffle) = &report.baffle {
        let _ = writeln!(out, "\n-- Baffle / Anti-Vignetting ({}) --", baffle.kind);
        let _ = writeln!(
            out,
            "Primary baffle hole:   {:.1} mm",
            baffle.primary_baffle_hole_diameter_mm
        );
        let _ = writeln!(
            out,
            "Midway diameter:       {:.1} mm",
            baffle.midway_baffle_diameter_mm
        );
        let _ = writeln!(
            out,
            "Secondary baffle tube: {:.1} mm",
            baffle.secondary_baffle_tube_diameter_mm
        );
        let _ = writeln!(
            out,
            "Linear obstruction:    {:.1}%",
            baffle.linear_obstruction_pct
        );
    }

    if let Some(support) = &report.support {
        let _ = writeln!(out, "\n-- Mirror Support ({}) --", support.support_scheme);
        let _ = writeln!(out, "Support points:  {}", support.n_points);
        let _ = writeln!(out, "RMS sag:         {:.3} nm", support.sag_nm);
        let _ = writeln!(
            out,
            "Sag budget:      {:.2}% of lambda/8",
            support.sag_pct_of_lambda_8
        );
    }

    if let Some(collimation) = &report.collimation {
        let _ = writeln!(out, "\n-- Collimation Sensitivity --");
        let _ = writeln!(
            out,
            "Coma per mm decenter:  {:.3} waves/mm",
            collimation.coma_per_mm_decenter_waves
        );
        let _ = writeln!(
            out,
            "Coma per arcsec tilt:  {:.3e} waves/arcsec",
            collimation.coma_per_arcsec_tilt_waves
        );
        let _ = writeln!(
            out,
            "Allowable decenter:    {:.3} mm (lambda/4)",
            collimation.allowable_decenter_mm
        );
        let _ = writeln!(
            out,
            "Allowable tilt:        {:.1} arcsec (lambda/4)",
            collimation.allowable_tilt_arcsec
        );
        let _ = writeln!(
            out,
            "Coma-free pivot:       {:.2} mm behind secondary vertex",
            collimation.coma_free_pivot_mm
        );
    }

    if let Some(seidel) = &report.seidel {
        let _ = writeln!(out, "\n-- Seidel Aberration Budget --");
        let _ = writeln!(out, "Field of view:   {:.2} deg", seidel.field_of_view_deg);
        let _ = writeln!(out, "S_II (coma):     {:.4e} mm", seidel.coma_s2);
        let _ = writeln!(out, "S_III (astig):   {:.4e} mm", seidel.astigmatism_s3);
        let _ = writeln!(
            out,
            "S_IV (field curv): {:.4e} mm",
            seidel.field_curvature_s4
        );
        let _ = writeln!(out, "Edge blur:       {:.2} arcsec", seidel.edge_blur_arcsec);
        if let Some(mode) = &seidel.conic_mode {
            let _ = writeln!(out, "Classification:  {mode}");
        }
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
mod tests;
