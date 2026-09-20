//! Phase C dispatch helpers: `collimation` (#5) and `seidel` (#7),
//! [[task-snapshot-v2-phaseC]] VC3 — the final integration chunk of the
//! 19-chunk v2 plan. Split out of `dispatch/mod.rs` to keep that file
//! under the 300-line limit.
//!
//! Both subsystems are two-mirror (Cassegrain-family) specific: they need
//! `SpecSheet.cassegrain` (secondary magnification `m`, `R2`, `K2` for
//! collimation; `R1`, `R2`, `K1`, `K2`, mirror separation `d` for seidel)
//! — a Newtonian's single primary or a refractor's lens train has neither
//! a secondary mirror to decenter/tilt nor a two-mirror Seidel sum to
//! evaluate, so both dispatch functions return `None` with an explanatory
//! note for any design without `sheet.cassegrain`. `seidel` additionally
//! needs `req.field_diam_mm` (the Seidel sums are evaluated at a specific
//! off-axis field angle) — same precondition pattern as `"baffle"`.

use super::fab_applicable_reason;
use crate::fabrication::{collimation, seidel};

/// Dispatch `collimation::compute` when `"collimation"` is in
/// `applicable` and `sheet.cassegrain` is present. Pulls `m`, `R2`, `K2`
/// from the already-computed v1 `CassegrainSpec` (same pattern
/// `dispatch.rs`'s `tolerance`/`baffle` blocks use for their Cassegrain
/// geometry inputs); `wavelength_nm` defaults to 550.0, same default as
/// the `tolerance` block above.
pub(super) fn dispatch_collimation(
    applicable: &[String],
    sheet: &crate::output::SpecSheet,
    req: &crate::request::Request,
    def: &crate::config::DesignDef,
    notes: &mut Vec<String>,
) -> Option<collimation::CollimationSpec> {
    if !applicable.iter().any(|s| s == "collimation") {
        notes.push(format!(
            "collimation: not applicable to design '{}' ({})",
            sheet.design_name,
            fab_applicable_reason(def)
        ));
        return None;
    }

    match &sheet.cassegrain {
        Some(cass) => {
            let wavelength_nm = req.wavelength_nm.unwrap_or(550.0);
            notes.push(
                "collimation: secondary decenter/tilt-to-coma sensitivity computed (C1 VERIFIED)"
                    .to_string(),
            );
            Some(collimation::compute(
                sheet.aperture_mm,
                sheet.focal_length_mm,
                cass.secondary_magnification,
                cass.r2_mm,
                cass.k2_secondary_conic,
                wavelength_nm,
            ))
        }
        None => {
            notes.push(format!(
                "collimation: not applicable to design '{}' (no Cassegrain-family geometry on this SpecSheet)",
                sheet.design_name
            ));
            None
        }
    }
}

/// Dispatch `seidel::compute` when `"seidel"` is in `applicable`,
/// `sheet.cassegrain` is present, AND `req.field_diam_mm` is given (the
/// Seidel sums are evaluated at a specific off-axis field angle, same
/// precondition pattern as `"baffle"`'s `dispatch.rs` block).
pub(super) fn dispatch_seidel(
    applicable: &[String],
    sheet: &crate::output::SpecSheet,
    req: &crate::request::Request,
    def: &crate::config::DesignDef,
    notes: &mut Vec<String>,
) -> Option<seidel::SeidelSpec> {
    if !applicable.iter().any(|s| s == "seidel") {
        notes.push(format!(
            "seidel: not applicable to design '{}' ({})",
            sheet.design_name,
            fab_applicable_reason(def)
        ));
        return None;
    }

    let cass = match &sheet.cassegrain {
        Some(cass) => cass,
        None => {
            notes.push(format!(
                "seidel: not applicable to design '{}' (no Cassegrain-family geometry on this SpecSheet)",
                sheet.design_name
            ));
            return None;
        }
    };

    let field_diam_mm = match req.field_diam_mm {
        Some(v) => v,
        None => {
            notes.push(
                "seidel: applicable to this design but no 'field_diam_mm' given in the request"
                    .to_string(),
            );
            return None;
        }
    };

    let wavelength_nm = req.wavelength_nm.unwrap_or(550.0);
    notes.push(
        "seidel: third-order Seidel aberration sums (coma/astigmatism/field curvature) computed (S2 VERIFIED)"
            .to_string(),
    );
    Some(seidel::compute(
        sheet.aperture_mm,
        sheet.focal_length_mm,
        cass.r1_mm,
        cass.r2_mm,
        cass.k1_primary_conic,
        cass.k2_secondary_conic,
        cass.mirror_separation_mm,
        field_diam_mm,
        wavelength_nm,
    ))
}
