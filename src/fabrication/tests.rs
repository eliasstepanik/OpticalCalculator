//! `#[cfg(test)] mod tests` body for `fabrication/mod.rs`, split out to stay
//! under the 300-line limit — same `mod.rs`/`tests.rs` sibling-file split
//! precedent as `config/` and `request/` (Chunk V1).

use super::*;
use crate::coating::{Coating, CoatingCatalog};
use crate::config::DesignDef;
use crate::output::SpecSheet;
use crate::request::Request;
use crate::substrate::{Substrate, SubstrateCatalog};

fn pyrex_catalog() -> SubstrateCatalog {
    SubstrateCatalog::from_substrates(vec![Substrate {
        name: "pyrex".to_string(),
        cte_ppm_per_k: 3.25,
        density_g_cm3: 2.23,
        cost_tier: 2,
        max_aperture_mm: 500.0,
    }])
}

fn enhanced_aluminum_catalog() -> CoatingCatalog {
    CoatingCatalog::from_coatings(vec![Coating {
        name: "enhanced_aluminum".to_string(),
        reflectivity_pct: 94.0,
    }])
}

/// Minimal Newtonian `SpecSheet` fixture — mirrors
/// `designs/newtonian.rs`'s test fixtures (basics only, no v2 fields
/// needed since `compute` doesn't read them yet).
fn newtonian_sheet() -> SpecSheet {
    SpecSheet {
        design_name: "newtonian".to_string(),
        aperture_mm: 200.0,
        focal_length_mm: 1200.0,
        f_ratio: 6.0,
        diffraction: None,
        fov: None,
        newtonian_secondary: None,
        cassegrain: None,
        refractor_prescription: None,
        notes: vec![],
    }
}

fn newtonian_def(fab_applicable: Option<Vec<String>>) -> DesignDef {
    DesignDef {
        pipeline: "newtonian".to_string(),
        elements: 2,
        kind: "reflector".to_string(),
        required: vec!["aperture_mm".to_string(), "f_ratio".to_string()],
        optional: vec![],
        outputs: vec!["basics".to_string()],
        conic: None,
        conic_mode: None,
        glasses: None,
        power_split: None,
        n_mirrors: Some(2),
        fab_applicable,
    }
}

fn refractor_def() -> DesignDef {
    DesignDef {
        pipeline: "refractor".to_string(),
        elements: 2,
        kind: "refractor".to_string(),
        required: vec!["aperture_mm".to_string(), "f_ratio".to_string()],
        optional: vec![],
        outputs: vec!["basics".to_string(), "prescription".to_string()],
        conic: None,
        conic_mode: None,
        glasses: Some(vec!["BK7".to_string()]),
        power_split: None,
        n_mirrors: None,
        fab_applicable: None,
    }
}

fn dummy_request() -> Request {
    Request {
        design: "newtonian".to_string(),
        aperture_mm: 200.0,
        f_ratio: 6.0,
        focuser_backfocus_mm: None,
        field_diam_mm: None,
        primary_f_ratio: None,
        backfocus_mm: None,
        eyepiece_fieldstop_mm: None,
        sensor_diag_mm: None,
        wavelength_nm: None,
        output_path: None,
        substrate: None,
        lightweight_fraction: None,
        coating: None,
        thickness_ratio: None,
    }
}

/// Acceptance check (Chunk V9): `compute()` runs without panic against a
/// sample Newtonian `SpecSheet`/`Request`/`DesignDef`, with `substrate`/
/// `coating` names present in the request and matching catalogs, and
/// returns `Ok(FabricationReport{..})` with `substrate`/`throughput`/
/// `tolerance` all actually computed (not just noted "pending").
#[test]
fn compute_dispatches_real_subsystems_for_applicable_design() {
    let sheet = newtonian_sheet();
    let mut req = dummy_request();
    req.substrate = Some("pyrex".to_string());
    req.coating = Some("enhanced_aluminum".to_string());
    let def = newtonian_def(Some(vec![
        "substrate".to_string(),
        "throughput".to_string(),
        "tolerance".to_string(),
    ]));

    let report = compute(
        &sheet,
        &req,
        &def,
        &pyrex_catalog(),
        &enhanced_aluminum_catalog(),
    )
    .expect("compute must not error");

    assert_eq!(report.design_name, "newtonian");
    let substrate = report.substrate.expect("substrate must be computed");
    assert_eq!(substrate.material, "pyrex");
    assert!((substrate.thickness_mm - 200.0 / 6.0).abs() < 1e-9);

    let throughput = report.throughput.expect("throughput must be computed");
    assert_eq!(throughput.coating, "enhanced_aluminum");
    // n_mirrors comes from def.n_mirrors (Some(2) on newtonian_def).
    assert!((throughput.system_throughput_pct - 88.36).abs() < 1e-9);

    let tolerance = report.tolerance.expect("tolerance must be computed");
    assert!((tolerance.surface_pv_tol_nm - 68.75).abs() < 1e-9);
}

/// `fab_applicable` absent -> infers reflector default (substrate +
/// tolerance + support + throughput all applicable) per
/// [[architecture-v2]] §2a. Without a `substrate`/`coating` name on the
/// request, both stay `None` with a "no name given" note; `tolerance`
/// needs no request field and is always computed when applicable.
#[test]
fn compute_infers_reflector_default_when_fab_applicable_absent() {
    let sheet = newtonian_sheet();
    let req = dummy_request();
    let def = newtonian_def(None);

    let report = compute(
        &sheet,
        &req,
        &def,
        &pyrex_catalog(),
        &enhanced_aluminum_catalog(),
    )
    .expect("compute must not error");

    assert!(report.substrate.is_none());
    assert!(report.throughput.is_none());
    assert!(report.tolerance.is_some());
    assert!(report
        .notes
        .iter()
        .any(|n| n.contains("substrate") && n.contains("no 'substrate' name given")));
    assert!(report
        .notes
        .iter()
        .any(|n| n.contains("throughput") && n.contains("no 'coating' name given")));
}

/// A design whose `fab_applicable` lists neither subsystem gets clean
/// "not applicable" notes, never an error — the never-error-on-N/A
/// contract ([[architecture-v2]] §0). Also locks in the follow-up V-2
/// wording fix: an explicit `fab_applicable` list that excludes a
/// subsystem must NOT blame `kind` in the note text.
#[test]
fn compute_notes_not_applicable_when_fab_applicable_excludes_subsystems() {
    let sheet = SpecSheet {
        design_name: "rc".to_string(),
        ..newtonian_sheet()
    };
    let req = dummy_request();
    let def = newtonian_def(Some(vec!["tolerance".to_string()]));

    let report = compute(
        &sheet,
        &req,
        &def,
        &pyrex_catalog(),
        &enhanced_aluminum_catalog(),
    )
    .expect("compute must not error");

    assert!(report.substrate.is_none());
    assert!(report.throughput.is_none());
    assert!(report.tolerance.is_some());
    assert!(report.notes[0].contains("not applicable to design 'rc'"));
    // V-2 fix: excluded via an explicit list, not "kind=" — this design's
    // `kind` is "reflector" (would normally include substrate/throughput),
    // so blaming kind here would be the exact bug follow-up V-2 describes.
    assert!(
        report.notes[0].contains("excluded by this design's fab_applicable list"),
        "note must name the real reason, not kind: {}",
        report.notes[0]
    );
    assert!(!report.notes[0].contains("kind="));
    assert!(report.notes[1].contains("not applicable to design 'rc'"));
    assert!(report.notes[1].contains("excluded by this design's fab_applicable list"));
}

/// Refractor default (no `fab_applicable`) -> substrate NOT applicable
/// (note correctly blames the kind-based default here, since there IS no
/// explicit list), throughput IS applicable ([[architecture-v2]] §2a
/// refractor rule), tolerance NOT applicable (refractor default omits it).
#[test]
fn compute_infers_refractor_default_throughput_only() {
    let sheet = SpecSheet {
        design_name: "achromat".to_string(),
        ..newtonian_sheet()
    };
    let req = dummy_request();
    let def = refractor_def();

    let report = compute(
        &sheet,
        &req,
        &def,
        &pyrex_catalog(),
        &enhanced_aluminum_catalog(),
    )
    .expect("compute must not error");

    assert!(report.notes[0].contains("not applicable"));
    assert!(report.notes[0].contains("kind='refractor' default does not include it"));
    assert!(report
        .notes
        .iter()
        .any(|n| n.contains("throughput") && n.contains("no 'coating' name given")));
    assert!(report.tolerance.is_none());
}

/// Unknown substrate name propagates `OptError::UnknownSubstrate`, not a
/// panic — the CLI error path (main.rs exits non-zero via `anyhow::Context`).
#[test]
fn compute_propagates_unknown_substrate_error() {
    let sheet = newtonian_sheet();
    let mut req = dummy_request();
    req.substrate = Some("moon_rock".to_string());
    let def = newtonian_def(Some(vec!["substrate".to_string()]));

    let err = compute(
        &sheet,
        &req,
        &def,
        &pyrex_catalog(),
        &enhanced_aluminum_catalog(),
    )
    .expect_err("unknown substrate must error, not panic");

    assert!(matches!(err, crate::error::OptError::UnknownSubstrate(name) if name == "moon_rock"));
}

/// Unknown coating name propagates `OptError::UnknownCoating`, not a panic.
#[test]
fn compute_propagates_unknown_coating_error() {
    let sheet = newtonian_sheet();
    let mut req = dummy_request();
    req.coating = Some("gold".to_string());
    let def = newtonian_def(Some(vec!["throughput".to_string()]));

    let err = compute(
        &sheet,
        &req,
        &def,
        &pyrex_catalog(),
        &enhanced_aluminum_catalog(),
    )
    .expect_err("unknown coating must error, not panic");

    assert!(matches!(err, crate::error::OptError::UnknownCoating(name) if name == "gold"));
}
