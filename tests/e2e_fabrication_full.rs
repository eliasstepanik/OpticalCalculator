//! Black-box end-to-end test for the v2 `--fabrication` pass's FULL
//! 8-subsystem report ([[task-snapshot-v2-phaseC]] VC3 — the final chunk
//! of the 19-chunk v2 plan). Confirms all 8 fabrication subsystems
//! (substrate, throughput, λ/8 + conic tolerance, baffle, support,
//! collimation, seidel) appear in one `--fabrication` run against the RC
//! example, alongside v1 non-regression.
//!
//! Same real-binary-invocation pattern as
//! `tests/e2e_fabrication_phase_a.rs`/`tests/e2e_fabrication_phase_b.rs`.

use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_optical-calculator"))
}

/// VC3 acceptance check 1: real binary run against
/// `configs/examples/rc-f8.toml --fabrication` shows ALL 8 subsystem
/// sections populated with real numbers, including the RC's
/// aplanatic/S_II≈0 headline.
#[test]
fn fabrication_flag_shows_all_8_subsystem_sections() {
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

    // v1 SpecSheet content still present ahead of the fabrication report.
    assert!(stdout.contains("Focal length:    1600.00 mm"), "{stdout}");

    // 1. Substrate / Mirror Blank
    assert!(stdout.contains("-- Substrate / Mirror Blank --"), "{stdout}");
    assert!(stdout.contains("Mass:            2.335 kg"), "{stdout}");

    // 2. Optical Throughput
    assert!(stdout.contains("-- Optical Throughput --"), "{stdout}");
    assert!(stdout.contains("System throughput: 88.36%"), "{stdout}");

    // 3. Fabrication Tolerance (lambda/8 surface figure + conic tolerance)
    assert!(stdout.contains("-- Fabrication Tolerance --"), "{stdout}");
    assert!(stdout.contains("Surface P-V tol: 68.75 nm (lambda/8)"), "{stdout}");
    assert!(stdout.contains("Primary conic tol (dK): 0.03802"), "{stdout}");
    assert!(stdout.contains("Secondary conic tol (dK): 0.45676"), "{stdout}");

    // 4. Baffle / Anti-Vignetting
    assert!(stdout.contains("-- Baffle / Anti-Vignetting (cassegrain) --"), "{stdout}");
    assert!(stdout.contains("Primary baffle hole:   36.9 mm"), "{stdout}");
    assert!(stdout.contains("Secondary baffle tube: 81.4 mm"), "{stdout}");

    // 5. Mirror Support
    assert!(stdout.contains("-- Mirror Support (9_point) --"), "{stdout}");
    assert!(stdout.contains("Support points:  9"), "{stdout}");

    // 6. Collimation Sensitivity (C1 VERIFIED, A2 RC worked example).
    assert!(stdout.contains("-- Collimation Sensitivity --"), "{stdout}");
    assert!(stdout.contains("Coma per mm decenter:  2.257 waves/mm"), "{stdout}");
    assert!(stdout.contains("Allowable decenter:    0.111 mm"), "{stdout}");
    assert!(stdout.contains("Allowable tilt:        139.4 arcsec"), "{stdout}");

    // 7. Seidel Aberration Budget (S2 VERIFIED) -- RC's S_II≈0/aplanatic
    // headline must be visibly present.
    assert!(stdout.contains("-- Seidel Aberration Budget --"), "{stdout}");
    assert!(stdout.contains("Classification:  aplanatic (coma-nulled)"), "{stdout}");

    // 8. Notes section confirming both Phase C subsystems dispatched
    // (not "not applicable").
    assert!(
        stdout.contains("collimation: secondary decenter/tilt-to-coma sensitivity computed (C1 VERIFIED)"),
        "{stdout}"
    );
    assert!(
        stdout.contains("seidel: third-order Seidel aberration sums (coma/astigmatism/field curvature) computed (S2 VERIFIED)"),
        "{stdout}"
    );
}

/// VC3 acceptance check 2: v1 non-regression -- without `--fabrication`,
/// the RC example's CLI output must be byte-for-byte identical to the
/// established v1 golden output (same golden string
/// `tests/e2e_fabrication_phase_a.rs`'s `no_flag_is_byte_identical_to_v1`
/// uses for the RC config), confirming VC3's dispatch wiring did not
/// touch the ungated v1 path.
#[test]
fn no_flag_rc_is_byte_identical_to_v1_golden() {
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
    assert_eq!(stdout, RC_V1_GOLDEN, "stdout is not byte-identical to the v1 golden output");
    assert!(
        output.stderr.is_empty(),
        "expected empty stderr, got: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
