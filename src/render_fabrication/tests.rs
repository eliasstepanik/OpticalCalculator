//! `#[cfg(test)] mod tests` body for `render_fabrication/mod.rs`.

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
        baffle: None,
        support: None,
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
