//! Black-box end-to-end tests for the shipped `optical-calculator` binary.
//!
//! These invoke the real compiled binary via `std::process::Command`
//! (the standard Rust integration-test pattern — see
//! <https://doc.rust-lang.org/cargo/reference/cargo-targets.html#integration-tests>)
//! against the three example configs from `configs/examples/` plus a
//! malformed-input error path. This proves the shipped CLI, run exactly as
//! a user would run it, produces correct output — it is not a re-test of
//! internal formula logic (already covered by the crate's unit tests).

use std::fs;
use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_optical-calculator"))
}

#[test]
fn newtonian_example_runs_and_prints_expected_values() {
    let output = bin()
        .arg("configs/examples/newtonian-f5-1000mm.toml")
        .output()
        .expect("failed to run binary");

    assert!(
        output.status.success(),
        "expected exit 0, got {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Ground-truth values re-verified by direct binary run (see
    // sessions/2026-09-19-optical-calculator-chunk17-cli-verification.md).
    assert!(stdout.contains("Focal length:    1000.00 mm"));
    assert!(stdout.contains("Aperture:        200.00 mm"));
    assert!(stdout.contains("Dawes limit:     0.58 arcsec"));
    assert!(stdout.contains("Minor axis:      60.96 mm"));
    assert!(stdout.contains("30.5% linear / 9.29% areal"));
}

#[test]
fn quintuplet_example_runs_and_prints_expected_values() {
    let output = bin()
        .arg("configs/examples/quintuplet-f6-apo.toml")
        .output()
        .expect("failed to run binary");

    assert!(
        output.status.success(),
        "expected exit 0, got {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("Focal length:    780.00 mm"));
    assert!(stdout.contains("Aperture:        130.00 mm"));
    assert!(stdout.contains("FPL53"));
    assert!(stdout.contains("BK7"));
}

#[test]
fn rc_example_runs_and_prints_expected_values() {
    let output = bin()
        .arg("configs/examples/rc-f8.toml")
        .output()
        .expect("failed to run binary");

    assert!(
        output.status.success(),
        "expected exit 0, got {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("Focal length:    1600.00 mm"));
    assert!(stdout.contains("Primary conic k1:    -1.1455"));
    assert!(stdout.contains("Secondary conic k2:  -6.5879"));
    assert!(stdout.contains("Back focus:          150.00 mm"));
}

#[test]
fn error_path_missing_required_field_exits_nonzero() {
    // Newtonian request missing the required `focuser_backfocus_mm` field.
    let dir = std::env::temp_dir().join(format!(
        "oc-e2e-{}-{}",
        std::process::id(),
        "missing-field"
    ));
    fs::create_dir_all(&dir).expect("create temp dir");
    let bad_config = dir.join("bad.toml");
    fs::write(
        &bad_config,
        "design = \"newtonian\"\naperture_mm = 200.0\nf_ratio = 5.0\n",
    )
    .expect("write temp config");

    let output = bin()
        .arg(&bad_config)
        .output()
        .expect("failed to run binary");

    assert!(
        !output.status.success(),
        "expected non-zero exit for malformed request, got exit 0"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("focuser_backfocus_mm") || stderr.contains("requires field"),
        "stderr did not contain a readable error message: {stderr}"
    );
    assert!(
        !stderr.contains("panicked at"),
        "stderr contained a panic backtrace instead of a clean error: {stderr}"
    );
    assert!(!stderr.contains("RUST_BACKTRACE"), "stderr: {stderr}");

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn output_flag_writes_valid_json() {
    let dir = std::env::temp_dir().join(format!("oc-e2e-{}-{}", std::process::id(), "json-out"));
    fs::create_dir_all(&dir).expect("create temp dir");
    let out_path = dir.join("out.json");

    let output = bin()
        .arg("configs/examples/newtonian-f5-1000mm.toml")
        .arg("--output")
        .arg(&out_path)
        .output()
        .expect("failed to run binary");

    assert!(
        output.status.success(),
        "expected exit 0, got {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let contents = fs::read_to_string(&out_path).expect("output json file should exist");
    let parsed: serde_json::Value =
        serde_json::from_str(&contents).expect("output file should be valid JSON");
    assert_eq!(parsed["design_name"], "newtonian");
    assert_eq!(parsed["focal_length_mm"], 1000.0);

    fs::remove_dir_all(&dir).ok();
}
