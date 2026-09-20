//! Black-box end-to-end tests for the F2/F3 quality-gate fixes
//! ([[optical-calculator-v2-quality-gate-review]]):
//!
//! - F2: refractor `--fabrication` runs must render a "not modeled" note
//!   for optical throughput instead of the misleading 100.00% figure.
//! - F3: `--fabrication --output <path>` must actually write the
//!   `FabricationReport` to a sibling `<path>.fabrication.<ext>` file
//!   (previously dead code — `--output` only wrote the v1 `SpecSheet`).
//!
//! Same real-binary-invocation pattern as `tests/e2e_fabrication_full.rs`.

use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_optical-calculator"))
}

/// F2 acceptance check: refractor design (based on
/// `quintuplet-f6-apo.toml`, n_mirrors=0) with `--fabrication` AND a
/// `coating` given (so `throughput` is exercised rather than skipped for
/// lack of a coating name) renders a "not modeled" note instead of
/// "System throughput: 100.00%" / "Mirror count: 0".
#[test]
fn refractor_fabrication_shows_not_modeled_throughput_note() {
    // Base example + `coating` so the throughput subsystem actually runs
    // (quintuplet's fab_applicable = ["throughput"], but throughput is
    // itself skipped-with-note when no coating is given).
    let mut request_path = std::env::temp_dir();
    request_path.push(format!(
        "optical_calculator_e2e_refractor_coating_{}.toml",
        std::process::id()
    ));
    std::fs::write(
        &request_path,
        "design = \"quintuplet\"\n\
         aperture_mm = 130.0\n\
         f_ratio = 6.0\n\
         coating = \"enhanced_aluminum\"\n",
    )
    .expect("must write temp request file");

    let output = bin()
        .arg(&request_path)
        .arg("--fabrication")
        .output()
        .expect("failed to run binary");

    std::fs::remove_file(&request_path).expect("cleanup temp request file must succeed");

    assert!(
        output.status.success(),
        "expected exit 0, got {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("-- Optical Throughput --"), "{stdout}");
    assert!(
        stdout.contains(
            "Optical Throughput: not modeled for refractor designs \
             (mirror-coating formula only; glass transmission losses \
             not calculated)"
        ),
        "{stdout}"
    );
    assert!(
        !stdout.contains("System throughput: 100.00%"),
        "misleading 100% figure must not appear: {stdout}"
    );
    assert!(
        !stdout.contains("Mirror count:    0"),
        "misleading mirror count must not appear: {stdout}"
    );
}

/// F3 acceptance check: `--fabrication --output <path>.json` writes BOTH
/// the v1 SpecSheet (at `<path>.json`, unchanged behavior) AND the
/// FabricationReport (at `<path>.fabrication.json`, new behavior), and
/// the fabrication file is valid, parseable JSON containing real report
/// data.
#[test]
fn output_flag_with_fabrication_writes_fabrication_report_file() {
    let mut spec_path = std::env::temp_dir();
    spec_path.push(format!(
        "optical_calculator_e2e_fab_output_{}.json",
        std::process::id()
    ));
    let mut fabrication_path = spec_path.clone();
    fabrication_path.set_extension("fabrication.json");
    // Cleanup from any prior failed run.
    let _ = std::fs::remove_file(&spec_path);
    let _ = std::fs::remove_file(&fabrication_path);

    let output = bin()
        .arg("configs/examples/rc-f8.toml")
        .arg("--fabrication")
        .arg("--output")
        .arg(&spec_path)
        .output()
        .expect("failed to run binary");

    assert!(
        output.status.success(),
        "expected exit 0, got {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    // v1 SpecSheet still written at the exact --output path, unchanged.
    assert!(spec_path.exists(), "SpecSheet output file must exist");
    let spec_contents = std::fs::read_to_string(&spec_path).expect("SpecSheet file must be readable");
    let spec_json: serde_json::Value =
        serde_json::from_str(&spec_contents).expect("SpecSheet file must be valid JSON");
    assert!(
        spec_json.get("aperture_mm").is_some(),
        "SpecSheet JSON must have aperture_mm field"
    );

    // FabricationReport written at the derived sibling path.
    assert!(
        fabrication_path.exists(),
        "FabricationReport output file must exist at {}",
        fabrication_path.display()
    );
    let fab_contents =
        std::fs::read_to_string(&fabrication_path).expect("FabricationReport file must be readable");
    let fab_json: serde_json::Value =
        serde_json::from_str(&fab_contents).expect("FabricationReport file must be valid JSON");
    assert_eq!(
        fab_json.get("design_name").and_then(|v| v.as_str()),
        Some("rc")
    );
    assert!(
        fab_json.get("throughput").is_some(),
        "FabricationReport JSON must have throughput field: {fab_contents}"
    );

    std::fs::remove_file(&spec_path).expect("cleanup spec file must succeed");
    std::fs::remove_file(&fabrication_path).expect("cleanup fabrication file must succeed");
}

/// F3 acceptance check (no-fabrication regression guard): without
/// `--fabrication`, `--output` must ONLY write the SpecSheet — no sibling
/// `.fabrication.*` file should appear.
#[test]
fn output_flag_without_fabrication_does_not_write_fabrication_file() {
    let mut spec_path = std::env::temp_dir();
    spec_path.push(format!(
        "optical_calculator_e2e_no_fab_output_{}.json",
        std::process::id()
    ));
    let mut fabrication_path = spec_path.clone();
    fabrication_path.set_extension("fabrication.json");
    let _ = std::fs::remove_file(&spec_path);
    let _ = std::fs::remove_file(&fabrication_path);

    let output = bin()
        .arg("configs/examples/rc-f8.toml")
        .arg("--output")
        .arg(&spec_path)
        .output()
        .expect("failed to run binary");

    assert!(output.status.success());
    assert!(spec_path.exists(), "SpecSheet output file must exist");
    assert!(
        !fabrication_path.exists(),
        "no fabrication file should be written when --fabrication is absent"
    );

    std::fs::remove_file(&spec_path).expect("cleanup spec file must succeed");
}
