//! `#[cfg(test)] mod tests` body for `fabrication/mod.rs`, split out to stay
//! under the 300-line limit — same `mod.rs`/`tests.rs` sibling-file split
//! precedent as `config/` and `request/` (Chunk V1).

use super::*;
use crate::config::DesignDef;
use crate::output::SpecSheet;
use crate::request::Request;

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

/// Acceptance check: `compute()` runs without panic against a sample
/// Newtonian `SpecSheet`/`Request`/`DesignDef` and returns
/// `Ok(FabricationReport{..})` with both subsystems marked applicable
/// (explicit `fab_applicable` list) and gracefully un-computed.
#[test]
fn compute_returns_ok_with_notes_for_applicable_design() {
    let sheet = newtonian_sheet();
    let req = dummy_request();
    let def = newtonian_def(Some(vec!["substrate".to_string(), "throughput".to_string()]));

    let report = compute(&sheet, &req, &def).expect("compute must not error");

    assert_eq!(report.design_name, "newtonian");
    assert!(report.substrate.is_none());
    assert!(report.throughput.is_none());
    assert_eq!(report.notes.len(), 2);
    assert!(report.notes[0].contains("pending chunk V4"));
    assert!(report.notes[1].contains("pending chunk V6"));
}

/// `fab_applicable` absent -> infers reflector default (substrate +
/// throughput both applicable) per [[architecture-v2]] §2a.
#[test]
fn compute_infers_reflector_default_when_fab_applicable_absent() {
    let sheet = newtonian_sheet();
    let req = dummy_request();
    let def = newtonian_def(None);

    let report = compute(&sheet, &req, &def).expect("compute must not error");

    assert!(report
        .notes
        .iter()
        .any(|n| n.contains("applicable") && n.contains("substrate") && !n.contains("not applicable")));
    assert!(report
        .notes
        .iter()
        .any(|n| n.contains("applicable") && n.contains("throughput") && !n.contains("not applicable")));
}

/// A design whose `fab_applicable` lists neither subsystem gets clean
/// "not applicable" notes, never an error — the never-error-on-N/A
/// contract ([[architecture-v2]] §0).
#[test]
fn compute_notes_not_applicable_when_fab_applicable_excludes_subsystems() {
    let sheet = SpecSheet {
        design_name: "rc".to_string(),
        ..newtonian_sheet()
    };
    let req = dummy_request();
    let def = newtonian_def(Some(vec!["tolerance".to_string()]));

    let report = compute(&sheet, &req, &def).expect("compute must not error");

    assert!(report.substrate.is_none());
    assert!(report.throughput.is_none());
    assert!(report.notes[0].contains("not applicable to design 'rc'"));
    assert!(report.notes[1].contains("not applicable to design 'rc'"));
}

/// Refractor default (no `fab_applicable`) -> substrate NOT applicable,
/// throughput IS applicable ([[architecture-v2]] §2a refractor rule).
#[test]
fn compute_infers_refractor_default_throughput_only() {
    let sheet = SpecSheet {
        design_name: "achromat".to_string(),
        ..newtonian_sheet()
    };
    let req = dummy_request();
    let def = refractor_def();

    let report = compute(&sheet, &req, &def).expect("compute must not error");

    assert!(report.notes[0].contains("not applicable"));
    assert!(report.notes[1].contains("pending chunk V6"));
}
