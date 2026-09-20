//! Phase A dispatch tests (substrate/throughput, Chunk V9) for
//! `fabrication::compute`. See `../fixtures.rs` for shared test fixtures.

use super::fixtures::*;
use crate::fabrication::compute;
use crate::output::SpecSheet;

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
