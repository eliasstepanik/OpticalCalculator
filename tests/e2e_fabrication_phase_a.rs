//! Black-box end-to-end tests for the v2 `--fabrication` pass (Phase A,
//! [[task-snapshot-v2-phaseA]] chunk V10 — the final Phase A chunk).
//!
//! Same pattern as v1's `tests/e2e.rs`: invoke the real compiled binary via
//! `std::process::Command`. Per
//! [[cli-v1-nonregression-differential-proof]]'s recommendation, the
//! critical non-regression test (`no_flag_is_byte_identical_to_v1`) uses a
//! golden-output byte-for-byte comparison rather than a loose substring
//! check, so any future chunk that silently changes v1 behavior on the
//! ungated path is caught immediately instead of relying on a point-in-time
//! differential sweep.

use std::fs;
use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_optical-calculator"))
}

/// Golden v1 output for `configs/examples/newtonian-f5-1000mm.toml`
/// (no `--fabrication`), captured by direct binary run 2026-09-20.
const NEWTONIAN_V1_GOLDEN: &str = "\
=== OpticalCalculator Spec Sheet ===
Design:          newtonian
Aperture:        200.00 mm
Focal length:    1000.00 mm
f-ratio:         f/5.00

-- Diffraction-limited performance --
Dawes limit:     0.58 arcsec
Airy disk diam:  6.71 um
Useful mag:      33.3x - 400.0x

-- Field of view --
Plate scale:     206.265 arcsec/mm
True FOV:        92.82 arcmin
Sensor FOV:      97.63 arcmin

-- Newtonian secondary (diagonal) --
Minor axis:      60.96 mm
Offset:          3.0480 mm
Obstruction:     30.5% linear / 9.29% areal
Primary conic:   -1.000

-- Notes --
* Predesign only — thin-mirror/paraxial similar-triangles approximation, not a fabrication-ready prescription (architecture.md §8 Q4).

";

/// Golden v1 output for `configs/examples/quintuplet-f6-apo.toml`
/// (no `--fabrication`), captured by direct binary run 2026-09-20.
const QUINTUPLET_V1_GOLDEN: &str = "\
=== OpticalCalculator Spec Sheet ===
Design:          quintuplet
Aperture:        130.00 mm
Focal length:    780.00 mm
f-ratio:         f/6.00

-- Diffraction-limited performance --
Dawes limit:     0.89 arcsec
Airy disk diam:  8.05 um
Useful mag:      21.7x - 260.0x

-- Field of view --
Plate scale:     264.442 arcsec/mm
True FOV:        119.00 arcmin
Sensor FOV:      190.84 arcmin

-- Refractor prescription --
Glass        Power (1/mm)      R1 (mm)      R2 (mm)
 1. FPL53        0.002025       267.13     -1147.45
 2. FPL53        0.002025     -1147.45      -182.27
 3. BK7         -0.001398      -182.27      -359.64
 4. F2           0.000029      -359.64      -353.64
 5. BK7         -0.001398      -353.64     -8210.76

-- Notes --
* predesign only, not fabrication-ready, thin-lens approximation
* element shapes default to symmetric (equiconvex/equiconcave), no spherical-aberration optimization
* quintuplet power-split uses a documented predesign heuristic (equal-power duplicate-glass grouping + mirror-symmetric pairing), not an optimized design.

";

/// Golden v1 output for `configs/examples/rc-f8.toml` (no `--fabrication`),
/// captured by direct binary run 2026-09-20.
const RC_V1_GOLDEN: &str = "\
=== OpticalCalculator Spec Sheet ===
Design:          rc
Aperture:        200.00 mm
Focal length:    1600.00 mm
f-ratio:         f/8.00

-- Diffraction-limited performance --
Dawes limit:     0.58 arcsec
Airy disk diam:  10.74 um
Useful mag:      33.3x - 400.0x

-- Field of view --
Plate scale:     128.916 arcsec/mm
True FOV:        58.01 arcmin
Sensor FOV:      61.02 arcmin

-- Cassegrain geometry --
Primary conic k1:    -1.1455
Secondary conic k2:  -6.5879
Back focus:          150.00 mm
Secondary mag:       2.667x
Mirror separation:   395.45 mm
R1 (primary):        1200.00 mm
R2 (secondary):      654.55 mm

-- Notes --
* predesign only, not fabrication-ready — thin-mirror conics/back-focus, no thickness/tolerance analysis (architecture.md §8 Q4)

";

#[test]
fn fabrication_flag_shows_rc_report() {
    let output = bin()
        .arg("configs/examples/rc-f8.toml")
        .arg("--fabrication")
        .output()
        .expect("failed to run binary");

    assert!(
        output.status.success(),
        "expected exit 0, got {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);

    // v1 SpecSheet content must still be present ahead of the fabrication
    // report (architecture-v2.md decision #4: fabrication report is
    // appended after, not instead of, the v1 sheet).
    assert!(stdout.contains("Focal length:    1600.00 mm"), "{stdout}");
    assert!(stdout.contains("Primary conic k1:    -1.1455"), "{stdout}");

    // Fabrication report content (worked examples per
    // [[task-snapshot-v2-phaseA]] V4/V6/V7 acceptance).
    assert!(
        stdout.contains("=== OpticalCalculator Fabrication Report ==="),
        "{stdout}"
    );
    assert!(stdout.contains("Mass:            2.335 kg"), "{stdout}");
    assert!(stdout.contains("System throughput: 88.36%"), "{stdout}");
    assert!(
        stdout.contains("Surface P-V tol: 68.75 nm (lambda/8)"),
        "{stdout}"
    );
}

/// THE critical non-regression test per
/// [[cli-v1-nonregression-differential-proof]]: without `--fabrication`,
/// CLI output must be byte-for-byte identical to v1's golden output, for
/// all three v1 example configs. A loose substring check would not catch a
/// regression in whitespace, note text, or a dropped/reordered section.
#[test]
fn no_flag_is_byte_identical_to_v1() {
    for (config, golden) in [
        ("configs/examples/newtonian-f5-1000mm.toml", NEWTONIAN_V1_GOLDEN),
        ("configs/examples/quintuplet-f6-apo.toml", QUINTUPLET_V1_GOLDEN),
        ("configs/examples/rc-f8.toml", RC_V1_GOLDEN),
    ] {
        let output = bin().arg(config).output().expect("failed to run binary");

        assert!(
            output.status.success(),
            "{config}: expected exit 0, got {:?}\nstderr: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout, golden,
            "{config}: stdout is not byte-identical to the v1 golden output"
        );
        assert!(
            output.stderr.is_empty(),
            "{config}: expected empty stderr, got: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn unknown_substrate_errors_gracefully() {
    let dir = std::env::temp_dir().join(format!(
        "oc-e2e-fab-{}-{}",
        std::process::id(),
        "bad-substrate"
    ));
    fs::create_dir_all(&dir).expect("create temp dir");
    let bad_config = dir.join("bad-substrate.toml");
    fs::write(
        &bad_config,
        "design = \"rc\"\n\
         aperture_mm = 200.0\n\
         f_ratio = 8.0\n\
         primary_f_ratio = 3.0\n\
         backfocus_mm = 150.0\n\
         substrate = \"unobtainium\"\n\
         coating = \"enhanced_aluminum\"\n",
    )
    .expect("write temp config");

    let output = bin()
        .arg(&bad_config)
        .arg("--fabrication")
        .output()
        .expect("failed to run binary");

    assert!(
        !output.status.success(),
        "expected non-zero exit for unknown substrate, got exit 0"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unobtainium") && stderr.contains("not found in catalog"),
        "stderr did not contain a readable error message: {stderr}"
    );
    assert!(
        !stderr.contains("panicked at"),
        "stderr contained a panic backtrace instead of a clean error: {stderr}"
    );
    assert!(!stderr.contains("RUST_BACKTRACE"), "stderr: {stderr}");

    fs::remove_dir_all(&dir).ok();
}

/// Per `main.rs`'s `--output` doc comment, `--output` applies to the v1
/// `SpecSheet` only — the fabrication report always goes to stdout, even
/// when `--output` is also passed. Verify both halves of that behavior:
/// the output file contains the v1 sheet (no fabrication data), and stdout
/// still carries the fabrication report.
#[test]
fn fabrication_report_via_output_flag_or_stdout_only() {
    let dir = std::env::temp_dir().join(format!(
        "oc-e2e-fab-{}-{}",
        std::process::id(),
        "output-flag"
    ));
    fs::create_dir_all(&dir).expect("create temp dir");
    let out_path = dir.join("out.json");

    let output = bin()
        .arg("configs/examples/rc-f8.toml")
        .arg("--fabrication")
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

    // The output file holds only the v1 SpecSheet JSON, no fabrication
    // fields (FabricationReport is a distinct type with its own renderer,
    // per architecture-v2.md decision #4).
    let contents = fs::read_to_string(&out_path).expect("output json file should exist");
    let parsed: serde_json::Value =
        serde_json::from_str(&contents).expect("output file should be valid JSON");
    assert_eq!(parsed["design_name"], "rc");
    assert_eq!(parsed["focal_length_mm"], 1600.0);
    assert!(
        parsed.get("substrate").is_none() && parsed.get("throughput").is_none(),
        "output file must not contain fabrication fields: {contents}"
    );

    // The fabrication report itself is only on stdout.
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("=== OpticalCalculator Fabrication Report ==="),
        "{stdout}"
    );
    assert!(stdout.contains("System throughput: 88.36%"), "{stdout}");

    fs::remove_dir_all(&dir).ok();
}
