//! `fabrication::compute` dispatch free-function + its private helpers.
//!
//! Split out of `fabrication/mod.rs` (V-8) to keep that file under the
//! 300-line limit once Phase B's `baffle`/`support`/conic-tolerance
//! wiring was added on top of Phase A's `substrate`/`throughput`/
//! surface-tolerance wiring (Chunk V9). `pub use dispatch::compute;` in
//! `mod.rs` re-exports this file's `compute` as `fabrication::compute`,
//! so callers (`main.rs`) see no path change.

use crate::fabrication::{
    baffle, substrate_mass, support, throughput, tolerance, FabricationReport,
};

/// Resolve which fabrication subsystem keys apply to `def`, per
/// [[architecture-v2]] §2a: an explicit `def.fab_applicable` list wins;
/// absent, infer a default from `def.kind` ("reflector"/"catadioptric" ->
/// substrate/tolerance/support/throughput; "refractor" -> throughput only).
fn resolve_fab_applicable(def: &crate::config::DesignDef) -> Vec<String> {
    match &def.fab_applicable {
        Some(list) => list.clone(),
        None => match def.kind.as_str() {
            "reflector" | "catadioptric" => vec![
                "substrate".to_string(),
                "tolerance".to_string(),
                "support".to_string(),
                "throughput".to_string(),
            ],
            "refractor" => vec!["throughput".to_string()],
            _ => vec![],
        },
    }
}

/// Format the "why not applicable" clause for a subsystem note.
///
/// **Follow-up V-2 fix (`session-state.md`):** the previous wording always
/// formatted `"not applicable ... (kind='{}')"`, blaming `def.kind` even
/// when the true reason was an explicit `def.fab_applicable` list that
/// simply omitted the subsystem — false for e.g. a reflector whose list
/// excludes `substrate` on purpose. This distinguishes the two real causes:
/// an explicit list that excludes the key, vs. the kind-based default
/// inference not including it.
fn fab_applicable_reason(def: &crate::config::DesignDef) -> String {
    match &def.fab_applicable {
        Some(_) => "excluded by this design's fab_applicable list".to_string(),
        None => format!("kind='{}' default does not include it", def.kind),
    }
}

/// Secondary mirror semi-aperture `y2` at the secondary's plane, mm — the
/// paraxial marginal-ray height from the primary's edge converging toward
/// prime focus, evaluated at axial distance `d` (mirror separation) from
/// the primary: `y2 = y1 · (1 − d/f1)` (similar triangles), with
/// `f1 = r1_mm / 2` (`CassegrainSpec.r1_mm = 2·f1`). Matches
/// [[formula-validation-v2]] T1's worked-example `y2≈34mm` for the A2 RC
/// (D=200, r1=1200, d=395.4545 -> y2=34.09mm).
fn secondary_semi_aperture_mm(aperture_mm: f64, r1_mm: f64, mirror_separation_mm: f64) -> f64 {
    let y1 = aperture_mm / 2.0;
    y1 * (1.0 - 2.0 * mirror_separation_mm / r1_mm)
}

/// Compute the fabrication report for an already-computed `SpecSheet`
/// ([[architecture-v2]] §1's `fabrication::compute` seam).
///
/// **Chunk V9 (Phase A) + V-8 (Phase B) dispatch.** For each subsystem
/// `resolve_fab_applicable` says is applicable, calls the matching
/// subsystem's `compute()` and records a `notes` entry; subsystems the
/// design doesn't support get a "not applicable" note
/// (`fab_applicable_reason`, follow-up V-2 fix). Never panics — an
/// unrecognized `def.kind` simply resolves to no applicable subsystems,
/// not an error (graceful-stub pattern, matches v1 Chunk 8).
///
/// **Phase B additions (V-8):**
/// - `"baffle"`: only meaningful for Cassegrain-family designs
///   (`sheet.cassegrain` populated, per [[v2-baffle-vignetting]]'s
///   Newtonian-out-of-scope note) AND `req.field_diam_mm` present
///   (`baffle::compute` needs a fully-illuminated field size — absent
///   either precondition, a note explains which).
/// - `"support"`: any design with a `req.substrate` name resolvable in
///   the catalog (support-sag needs density/Young's modulus from the
///   substrate, same precondition as `"substrate"` itself) — uses
///   `req.support_scheme`/`req.support_n_points`, defaulting to
///   `"9_point"`/`9` when absent.
/// - `"tolerance"`: now calls `tolerance::compute_phase_b_conic` when
///   `sheet.cassegrain` is available (every current design has at least a
///   primary conic via this path or a Newtonian's fixed `-1.0`; Newtonian
///   has no `CassegrainSpec`, so it falls back to `compute_phase_a`, the
///   V9 Phase-A-only behavior) — replaces the old "T1 pending" note,
///   fixing follow-up V-9.
///
/// `substrate_catalog`/`coating_catalog` are only consulted when the
/// corresponding subsystem is applicable AND the request supplies the
/// matching name (`req.substrate`/`req.coating`); a missing name is
/// reported as a note, not an error, and an unknown catalog name
/// propagates `OptError::UnknownSubstrate`/`UnknownCoating` (main.rs exits
/// non-zero, no panic).
pub fn compute(
    sheet: &crate::output::SpecSheet,
    req: &crate::request::Request,
    def: &crate::config::DesignDef,
    substrate_catalog: &crate::substrate::SubstrateCatalog,
    coating_catalog: &crate::coating::CoatingCatalog,
) -> Result<FabricationReport, crate::error::OptError> {
    let applicable = resolve_fab_applicable(def);
    let mut notes = Vec::new();

    let substrate = if applicable.iter().any(|s| s == "substrate") {
        match &req.substrate {
            Some(name) => {
                let mat = substrate_catalog.get(name)?;
                let thickness_ratio = req.thickness_ratio.unwrap_or(6.0);
                let lightweight_fraction = req.lightweight_fraction.unwrap_or(0.0);
                Some(substrate_mass::compute(
                    sheet.aperture_mm,
                    thickness_ratio,
                    lightweight_fraction,
                    mat,
                ))
            }
            None => {
                notes.push(
                    "substrate mass: applicable to this design but no 'substrate' name given in the request"
                        .to_string(),
                );
                None
            }
        }
    } else {
        notes.push(format!(
            "substrate mass: not applicable to design '{}' ({})",
            sheet.design_name,
            fab_applicable_reason(def)
        ));
        None
    };

    let throughput = if applicable.iter().any(|s| s == "throughput") {
        match &req.coating {
            Some(name) => {
                let coat = coating_catalog.get(name)?;
                let n_mirrors = def.n_mirrors.unwrap_or(0);
                Some(throughput::compute(coat, n_mirrors))
            }
            None => {
                notes.push(
                    "throughput: applicable to this design but no 'coating' name given in the request"
                        .to_string(),
                );
                None
            }
        }
    } else {
        notes.push(format!(
            "throughput: not applicable to design '{}' ({})",
            sheet.design_name,
            fab_applicable_reason(def)
        ));
        None
    };

    let tolerance = if applicable.iter().any(|s| s == "tolerance") {
        let wavelength_nm = req.wavelength_nm.unwrap_or(550.0);
        match &sheet.cassegrain {
            Some(cass) => {
                let y1_mm = sheet.aperture_mm / 2.0;
                let y2_mm =
                    secondary_semi_aperture_mm(sheet.aperture_mm, cass.r1_mm, cass.mirror_separation_mm);
                notes.push(
                    "tolerance: λ/8 surface figure + per-surface conic-constant tolerance (T1 VERIFIED)"
                        .to_string(),
                );
                Some(tolerance::compute_phase_b_conic(
                    wavelength_nm,
                    cass.k1_primary_conic,
                    cass.r1_mm,
                    y1_mm,
                    Some(cass.k2_secondary_conic),
                    Some(cass.r2_mm),
                    Some(y2_mm),
                    def.conic_mode.clone(),
                ))
            }
            None => {
                notes.push("tolerance: λ/8 surface figure only — no conic geometry available for this design's pipeline".to_string());
                Some(tolerance::compute_phase_a(wavelength_nm))
            }
        }
    } else {
        notes.push(format!(
            "tolerance: not applicable to design '{}' ({})",
            sheet.design_name,
            fab_applicable_reason(def)
        ));
        None
    };

    let baffle = if applicable.iter().any(|s| s == "baffle") {
        match &sheet.cassegrain {
            Some(cass) => match baffle::compute(
                sheet.aperture_mm,
                sheet.focal_length_mm,
                cass.back_focus_mm,
                cass.mirror_separation_mm,
                req.field_diam_mm,
            ) {
                Some(spec) => {
                    notes.push("baffle: Cassegrain-family diameter profile computed (B1 VERIFIED)".to_string());
                    Some(spec)
                }
                None => {
                    notes.push(
                        "baffle: applicable to this design but no 'field_diam_mm' given in the request"
                            .to_string(),
                    );
                    None
                }
            },
            None => {
                notes.push(format!(
                    "baffle: not applicable to design '{}' (no Cassegrain-family geometry on this SpecSheet)",
                    sheet.design_name
                ));
                None
            }
        }
    } else {
        notes.push(format!(
            "baffle: not applicable to design '{}' ({})",
            sheet.design_name,
            fab_applicable_reason(def)
        ));
        None
    };

    let support = if applicable.iter().any(|s| s == "support") {
        match &req.substrate {
            Some(name) => {
                let mat = substrate_catalog.get(name)?;
                let scheme = req.support_scheme.as_deref().unwrap_or("9_point");
                let n_points = req.support_n_points.unwrap_or(9);
                let thickness_mm = req
                    .thickness_ratio
                    .map(|ratio| sheet.aperture_mm / ratio)
                    .unwrap_or(sheet.aperture_mm / 6.0);
                let spec = support::compute(sheet.aperture_mm, thickness_mm, mat, scheme, n_points)?;
                notes.push(format!(
                    "support: {scheme} scheme, {n_points} points (S1 VERIFIED)"
                ));
                Some(spec)
            }
            None => {
                notes.push(
                    "support: applicable to this design but no 'substrate' name given in the request"
                        .to_string(),
                );
                None
            }
        }
    } else {
        notes.push(format!(
            "support: not applicable to design '{}' ({})",
            sheet.design_name,
            fab_applicable_reason(def)
        ));
        None
    };

    Ok(FabricationReport {
        design_name: sheet.design_name.clone(),
        substrate,
        throughput,
        tolerance,
        baffle,
        support,
        notes,
    })
}
