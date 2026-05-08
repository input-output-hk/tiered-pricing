//! M5 — suite-level pricing-event-stream determinism goldens.
//!
//! For each of the 7 phase-2 suites, the canonical baseline (job,
//! seed=1) pair is run end-to-end and its
//! `RunSummary.pricing_event_stream_sha256` is asserted equal to a
//! committed golden under
//! `parameters/phase-2-sweep/suites/.goldens/<suite>.sha256`.
//!
//! These tests are slow (each baseline run is a 200-slot
//! single-producer simulation) and `#[ignore]`'d by default so the
//! standard `cargo test` cycle stays fast. Run them via:
//!
//! ```text
//! cd sim-rs && cargo test --release -- --ignored determinism
//! ```
//!
//! Setting the environment variable `UPDATE_GOLDENS=1` writes the
//! freshly-computed hash to the goldens file instead of asserting
//! against it. Use after intentional simulator changes (e.g.
//! controller-arithmetic refactor); commit the result and tag the
//! branch.
//!
//! Output artefacts (time-series CSV, diagnostics, run summary) go
//! to a per-test `tempfile::TempDir` so they don't pollute
//! `sim-rs/output/`.

use std::path::{Path, PathBuf};

use sim_cli::{runner, suite::Suite};

/// `sim-rs/` root, computed from cargo's `CARGO_MANIFEST_DIR`
/// (`sim-rs/sim-cli/`). Suite YAMLs reference paths like
/// `parameters/phase-2-sweep/...` relative to the `sim-rs/` working
/// directory, so the test must rebase those onto absolute paths.
fn sim_rs_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sim-cli/Cargo.toml lives under sim-rs/")
        .to_path_buf()
}

fn suite_yaml_path(suite_name: &str) -> PathBuf {
    sim_rs_root()
        .join("parameters/phase-2-sweep/suites")
        .join(format!("{suite_name}.yaml"))
}

fn goldens_path(suite_name: &str) -> PathBuf {
    sim_rs_root()
        .join("parameters/phase-2-sweep/suites/.goldens")
        .join(format!("{suite_name}.sha256"))
}

/// Rebase every relative path inside a freshly-loaded `Suite` onto
/// `sim-rs/` so paths resolve regardless of cargo's working dir.
fn rebase_suite_paths(suite: &mut Suite) {
    let root = sim_rs_root();
    let abs = |p: &Path| -> PathBuf {
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            root.join(p)
        }
    };
    suite.default_topology = abs(&suite.default_topology);
    suite.default_protocol = abs(&suite.default_protocol);
    suite.default_demand = abs(&suite.default_demand);
    for job in &mut suite.jobs {
        job.pricing = abs(&job.pricing);
        if let Some(p) = &job.overrides.topology {
            job.overrides.topology = Some(abs(p));
        }
        if let Some(p) = &job.overrides.protocol {
            job.overrides.protocol = Some(abs(p));
        }
        if let Some(p) = &job.overrides.demand {
            job.overrides.demand = Some(abs(p));
        }
    }
}

/// Run the suite's baseline (job, seed) and either assert the result
/// matches the committed golden, or — under `UPDATE_GOLDENS=1` — write
/// the freshly-computed hash to disk.
///
/// Output artefacts go to `tempdir`; the goldens file is the only
/// repository-tracked artefact this test cares about.
fn run_baseline_and_check_golden(suite_name: &str, baseline_job: &str, seed: u64) {
    let suite_path = suite_yaml_path(suite_name);
    let mut suite = Suite::load(&suite_path)
        .unwrap_or_else(|e| panic!("loading suite {}: {e}", suite_path.display()));
    rebase_suite_paths(&mut suite);

    // Redirect output to a per-test tempdir so the test never writes
    // to sim-rs/output/.
    let tmp = tempfile::tempdir().expect("creating tempdir");
    suite.output_dir = tmp.path().to_path_buf();

    let job_idx = suite
        .jobs
        .iter()
        .position(|j| j.name == baseline_job)
        .unwrap_or_else(|| {
            panic!(
                "baseline job '{baseline_job}' not found in suite '{suite_name}'; \
                 known jobs: {:?}",
                suite.jobs.iter().map(|j| &j.name).collect::<Vec<_>>()
            )
        });

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("building tokio runtime");
    let summary = runtime
        .block_on(async { runner::run_job(&suite, job_idx, seed).await })
        .unwrap_or_else(|e| panic!("running {suite_name}/{baseline_job} seed={seed}: {e:#}"));
    let fresh = summary.pricing_event_stream_sha256;
    assert_eq!(
        fresh.len(),
        64,
        "freshly-computed hash for {suite_name}/{baseline_job} seed={seed} \
         is not a 64-char hex digest; got {fresh:?}"
    );

    let goldens_file = goldens_path(suite_name);
    if std::env::var("UPDATE_GOLDENS").is_ok() {
        std::fs::create_dir_all(goldens_file.parent().unwrap()).unwrap();
        let line = format!("{baseline_job} {seed} {fresh}\n");
        std::fs::write(&goldens_file, &line).unwrap_or_else(|e| {
            panic!("writing golden {}: {e}", goldens_file.display())
        });
        eprintln!(
            "UPDATE_GOLDENS=1 wrote {}: {fresh}",
            goldens_file.display()
        );
        return;
    }

    let stored = std::fs::read_to_string(&goldens_file).unwrap_or_else(|e| {
        panic!(
            "reading golden {}: {e}; run with UPDATE_GOLDENS=1 to generate",
            goldens_file.display()
        )
    });
    let parsed: Vec<&str> = stored.split_whitespace().collect();
    assert!(
        parsed.len() >= 3,
        "malformed golden {}: expected '<job> <seed> <hash>'; got {stored:?}",
        goldens_file.display()
    );
    let stored_job = parsed[0];
    let stored_seed: u64 = parsed[1]
        .parse()
        .unwrap_or_else(|_| panic!("non-integer seed in golden {}", goldens_file.display()));
    let stored_hash = parsed[2];
    assert_eq!(
        stored_job, baseline_job,
        "golden {} pins job '{stored_job}' but the test expects baseline '{baseline_job}'",
        goldens_file.display()
    );
    assert_eq!(
        stored_seed, seed,
        "golden {} pins seed {stored_seed} but the test expects {seed}",
        goldens_file.display()
    );
    assert_eq!(
        stored_hash.len(),
        64,
        "golden {} is not a 64-char hex digest; got {stored_hash:?}",
        goldens_file.display()
    );
    assert_eq!(
        fresh, stored_hash,
        "{suite_name}/{baseline_job} seed={seed} hash drifted: \
         fresh={fresh} stored={stored_hash}. \
         Re-run with UPDATE_GOLDENS=1 if the change is intentional."
    );
}

#[test]
#[ignore]
fn determinism_phase_2_eip1559_robustness() {
    run_baseline_and_check_golden(
        "phase-2-eip1559-robustness",
        "d8_target0.5_window32",
        1,
    );
}

#[test]
#[ignore]
fn determinism_phase_2_eip1559_smoothing() {
    run_baseline_and_check_golden("phase-2-eip1559-smoothing", "window32", 1);
}

#[test]
#[ignore]
fn determinism_phase_2_priority_only_rb_reserved() {
    run_baseline_and_check_golden(
        "phase-2-priority-only-rb-reserved",
        "multiplier_x4",
        1,
    );
}

#[test]
#[ignore]
fn determinism_phase_2_priority_only_unreserved() {
    run_baseline_and_check_golden(
        "phase-2-priority-only-unreserved",
        "multiplier_x4",
        1,
    );
}

#[test]
#[ignore]
fn determinism_phase_2_two_lane_both_dynamic() {
    run_baseline_and_check_golden(
        "phase-2-two-lane-both-dynamic",
        "partitioned_x4",
        1,
    );
}

#[test]
#[ignore]
fn determinism_phase_2_rb_scarcity() {
    run_baseline_and_check_golden("phase-2-rb-scarcity", "rb_baseline", 1);
}

#[test]
#[ignore]
fn determinism_phase_2_urgency_inversion() {
    run_baseline_and_check_golden(
        "phase-2-urgency-inversion",
        "correctly_priced",
        1,
    );
}
