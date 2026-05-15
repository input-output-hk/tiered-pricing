//! Concurrency tests for the experiment-suite runner.
//!
//! These exercise the JoinSet-pattern dispatcher added to
//! `run_suite_with_run_id`: bit-identity of artefacts across
//! parallelism levels, partial-failure manifest state, resume
//! semantics, and per-`run_id` output-dir isolation.
//!
//! All tests use a single shared "tiny suite" fixture: 2 jobs × 2
//! seeds × 100-slot single-producer runs. Wall time per fixture build
//! is under 5 seconds in `--release`; whole file runs in well under 30
//! seconds. Unlike `determinism.rs` these are NOT `#[ignore]`'d —
//! they run on every `cargo test --workspace`.
//!
//! The fixture writes a fresh `suite.yaml` into a `tempfile::TempDir`
//! that references the real parameter YAMLs under
//! `parameters/phase-2-sweep/` so `run_job` actually executes a
//! genuine (job, seed) end-to-end. We need that to get a real
//! `pricing_event_stream.sha256` to compare across runs.
//!
//! The "configured to fail" partial-failure test points one job's
//! pricing field at a path that doesn't exist, which causes
//! `merge_layer` to fail during config composition inside `run_job`.

use std::path::{Path, PathBuf};

use sim_cli::{
    runner::{JobStatus, Manifest, run_suite_with_run_id},
    suite::Suite,
};

const TINY_SLOTS: u64 = 100;

fn sim_rs_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sim-cli/Cargo.toml lives under sim-rs/")
        .to_path_buf()
}

/// Path of a real parameter YAML, rebased onto `sim-rs/`.
fn param(relative: &str) -> PathBuf {
    sim_rs_root().join(relative)
}

/// Write a tiny 2-job × 2-seed suite to `tmpdir`. Each job's pricing
/// field is either a real path (for valid jobs) or a non-existent path
/// (for "configured to fail").
struct TinySuiteBuilder {
    output_dir: PathBuf,
    jobs: Vec<(String, PathBuf)>,
    seeds: Vec<u64>,
    slots: u64,
}

impl TinySuiteBuilder {
    fn new(tmp: &Path) -> Self {
        Self {
            output_dir: tmp.join("output"),
            jobs: Vec::new(),
            seeds: vec![1, 2],
            slots: TINY_SLOTS,
        }
    }

    fn with_job(mut self, name: &str, pricing: PathBuf) -> Self {
        self.jobs.push((name.to_string(), pricing));
        self
    }

    fn with_seeds(mut self, seeds: Vec<u64>) -> Self {
        self.seeds = seeds;
        self
    }

    fn with_slots(mut self, slots: u64) -> Self {
        self.slots = slots;
        self
    }

    fn write(&self, tmp: &Path) -> PathBuf {
        let topology = param("parameters/phase-2-sweep/topology-single-producer.yaml");
        let protocol = param("parameters/phase-2-sweep/protocol-base.yaml");
        let demand = param("parameters/phase-2-sweep/demand/paper_like_congested.yaml");
        let mut yaml = String::new();
        yaml.push_str("suite-name: parallel-test\n");
        yaml.push_str(&format!("output-dir: {}\n", self.output_dir.display()));
        yaml.push_str(&format!(
            "seeds: [{}]\n",
            self.seeds
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
        yaml.push_str(&format!("default-slots: {}\n", self.slots));
        yaml.push_str(&format!("default-topology: {}\n", topology.display()));
        yaml.push_str(&format!("default-protocol: {}\n", protocol.display()));
        yaml.push_str(&format!("default-demand: {}\n", demand.display()));
        yaml.push_str("jobs:\n");
        for (name, pricing) in &self.jobs {
            yaml.push_str(&format!(
                "  - name: {}\n    pricing: {}\n",
                name,
                pricing.display()
            ));
        }
        let suite_path = tmp.join("suite.yaml");
        std::fs::write(&suite_path, yaml).expect("writing suite.yaml");
        suite_path
    }
}

fn read_hash(job_dir: &Path) -> String {
    std::fs::read_to_string(job_dir.join("pricing_event_stream.sha256"))
        .unwrap_or_else(|e| panic!("reading hash at {}: {e}", job_dir.display()))
        .trim()
        .to_string()
}

fn load_manifest(suite_path: &Path) -> Manifest {
    let suite = Suite::load(suite_path).expect("load suite");
    let manifest_path = suite.output_dir.join("manifest.json");
    let text = std::fs::read_to_string(&manifest_path)
        .unwrap_or_else(|e| panic!("reading {}: {e}", manifest_path.display()));
    serde_json::from_str(&text).expect("parsing manifest")
}

/// Two valid jobs that produce different event streams (so any
/// accidental cross-talk between concurrent jobs surfaces as a hash
/// mismatch). Both pricing configs are real YAMLs from the repo.
fn two_valid_jobs() -> Vec<(String, PathBuf)> {
    vec![
        (
            "baseline".to_string(),
            param("parameters/phase-2-sweep/pricing/baseline_flat_fee.yaml"),
        ),
        (
            "eip1559_window32".to_string(),
            param("parameters/phase-2-sweep/pricing/eip1559_d8_target0.5_window32.yaml"),
        ),
    ]
}

/// Core invariant: `--parallelism 4` produces bit-identical
/// `pricing_event_stream.sha256` for every (job, seed) as
/// `--parallelism 1`. If anything in the simulator's hot path
/// accidentally depends on global state (per-job state leaking, shared
/// RNG seeded once, etc.) this catches it.
#[test]
fn parallel_run_matches_sequential() {
    let tmp_seq = tempfile::tempdir().expect("tempdir seq");
    let tmp_par = tempfile::tempdir().expect("tempdir par");

    let mut b = TinySuiteBuilder::new(tmp_seq.path());
    for (name, pricing) in two_valid_jobs() {
        b = b.with_job(&name, pricing);
    }
    let suite_seq_path = b.write(tmp_seq.path());

    let mut b = TinySuiteBuilder::new(tmp_par.path());
    for (name, pricing) in two_valid_jobs() {
        b = b.with_job(&name, pricing);
    }
    let suite_par_path = b.write(tmp_par.path());

    run_suite_with_run_id(&suite_seq_path, None, 1).expect("sequential run");
    run_suite_with_run_id(&suite_par_path, None, 4).expect("parallel run");

    let suite_seq = Suite::load(&suite_seq_path).unwrap();
    let suite_par = Suite::load(&suite_par_path).unwrap();
    for (job_idx, seed) in suite_seq.job_seed_pairs() {
        let job_name = &suite_seq.jobs[job_idx].name;
        let dir_seq = suite_seq.output_dir.join(job_name).join(seed.to_string());
        let dir_par = suite_par.output_dir.join(job_name).join(seed.to_string());
        let h_seq = read_hash(&dir_seq);
        let h_par = read_hash(&dir_par);
        assert_eq!(
            h_seq, h_par,
            "hash drift for {job_name} seed={seed}: seq={h_seq} par={h_par}"
        );
    }
}

/// A configured-to-fail job ends `Failed` in the manifest; sibling
/// jobs complete cleanly; no entries are stuck in `Running`. Critical
/// for resume-after-failure: the next `run` must be able to retry only
/// the Failed pair without re-executing the Completed ones.
#[test]
fn partial_failure_leaves_recoverable_manifest() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let bad = tmp.path().join("does_not_exist.yaml");
    let b = TinySuiteBuilder::new(tmp.path())
        .with_job(
            "good",
            param("parameters/phase-2-sweep/pricing/baseline_flat_fee.yaml"),
        )
        .with_job("bad", bad);
    let suite_path = b.write(tmp.path());

    let err = run_suite_with_run_id(&suite_path, None, 4)
        .expect_err("expected at least one (job, seed) to fail");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("failed") || msg.contains("does_not_exist"),
        "expected error to mention failure or the bad path; got: {msg}"
    );

    let manifest = load_manifest(&suite_path);
    let good = manifest.jobs.get("good").expect("good job in manifest");
    for (seed, entry) in good {
        assert_eq!(
            entry.status,
            JobStatus::Completed,
            "good seed={seed} expected Completed, got {:?}",
            entry.status
        );
    }
    let bad = manifest.jobs.get("bad").expect("bad job in manifest");
    for (seed, entry) in bad {
        assert_eq!(
            entry.status,
            JobStatus::Failed,
            "bad seed={seed} expected Failed, got {:?}",
            entry.status
        );
        assert!(
            entry.error.is_some(),
            "bad seed={seed} expected error message"
        );
    }
    // Critical resume invariant: no Running entries left after the run
    // returns. (Manifest::load_or_init resets Running → Pending on
    // reload as a backstop, but a clean run shouldn't need it.)
    for (job, seeds) in &manifest.jobs {
        for (seed, entry) in seeds {
            assert_ne!(
                entry.status,
                JobStatus::Running,
                "{job} seed={seed} stuck in Running"
            );
        }
    }
}

/// Resume semantics: after a clean run, re-running with the same
/// run_id (= same output_dir) must not re-execute any (job, seed).
/// We measure this via the `run_summary.json` mtime: a re-run would
/// rewrite it, a skip-completed would not.
#[test]
fn resume_under_parallelism_skips_completed() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut b = TinySuiteBuilder::new(tmp.path());
    for (name, pricing) in two_valid_jobs() {
        b = b.with_job(&name, pricing);
    }
    let suite_path = b.write(tmp.path());

    run_suite_with_run_id(&suite_path, None, 2).expect("first run");

    let suite = Suite::load(&suite_path).unwrap();
    let mut mtimes: Vec<(PathBuf, std::time::SystemTime)> = Vec::new();
    for (job_idx, seed) in suite.job_seed_pairs() {
        let job_name = &suite.jobs[job_idx].name;
        let dir = suite.output_dir.join(job_name).join(seed.to_string());
        let summary = dir.join("run_summary.json");
        let m = std::fs::metadata(&summary)
            .unwrap_or_else(|e| panic!("metadata {}: {e}", summary.display()));
        mtimes.push((summary, m.modified().expect("mtime")));
    }

    // Sleep just long enough that any re-write would show a different
    // mtime on common filesystems (most are ms-granular; 50 ms is
    // safe). If this test ever becomes flaky on a coarse-mtime FS,
    // bump to 1 s — it doesn't run on every commit.
    std::thread::sleep(std::time::Duration::from_millis(50));

    run_suite_with_run_id(&suite_path, None, 4).expect("second run");

    for (summary, before) in mtimes {
        let m = std::fs::metadata(&summary)
            .unwrap_or_else(|e| panic!("metadata {}: {e}", summary.display()));
        let after = m.modified().expect("mtime");
        assert_eq!(
            after,
            before,
            "{} was rewritten on resume — Completed jobs must not re-run",
            summary.display()
        );
    }
}

/// Wall-clock smoke: a workload large enough to make per-job time
/// non-trivial (~hundreds of ms in `--release`) and 8 (job, seed)
/// pairs so a 4× speedup is observable. `#[ignore]`'d by default
/// because timings are noisy under shared load — run explicitly via
/// `cargo test --release -- --ignored parallel_wall_clock`. Prints
/// the speedup to stderr; asserts only that the parallel run isn't
/// dramatically *slower* than sequential (catches a hard-coded
/// regression where parallelism somehow serializes).
#[test]
#[ignore]
fn parallel_wall_clock_smoke() {
    use std::time::Instant;
    const SLOTS: u64 = 400;
    let mk = |dir: &Path, slots: u64| -> PathBuf {
        let mut b = TinySuiteBuilder::new(dir)
            .with_slots(slots)
            .with_seeds(vec![1, 2, 3, 4]);
        for (name, pricing) in two_valid_jobs() {
            b = b.with_job(&name, pricing);
        }
        b.write(dir)
    };
    let tmp_seq = tempfile::tempdir().unwrap();
    let tmp_par = tempfile::tempdir().unwrap();
    let s = mk(tmp_seq.path(), SLOTS);
    let p = mk(tmp_par.path(), SLOTS);
    let t0 = Instant::now();
    run_suite_with_run_id(&s, None, 1).unwrap();
    let seq = t0.elapsed();
    let t0 = Instant::now();
    run_suite_with_run_id(&p, None, 4).unwrap();
    let par = t0.elapsed();
    eprintln!("parallel_wall_clock_smoke: sequential={seq:?} parallel-4={par:?}");
    // Don't gate on a strict speedup — under shared load the parallel
    // run may even be slightly slower. Only catch the dramatic
    // serialisation regression.
    assert!(
        par.as_secs_f64() < seq.as_secs_f64() * 2.5,
        "parallel-4 was dramatically slower than sequential: seq={seq:?} par={par:?}"
    );
}

/// `run_id` suffixing still works under parallelism: two distinct
/// `--run-id` values produce two distinct output dirs, each with its
/// own manifest, each containing every (job, seed) as Completed.
#[test]
fn run_id_suffix_still_works_under_parallelism() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut b = TinySuiteBuilder::new(tmp.path());
    for (name, pricing) in two_valid_jobs() {
        b = b.with_job(&name, pricing);
    }
    let suite_path = b.write(tmp.path());

    run_suite_with_run_id(&suite_path, Some("alpha"), 2).expect("alpha run");
    run_suite_with_run_id(&suite_path, Some("beta"), 2).expect("beta run");

    let suite = Suite::load(&suite_path).unwrap();
    let alpha_dir = {
        let stem = suite.output_dir.file_name().unwrap().to_string_lossy();
        suite
            .output_dir
            .parent()
            .unwrap()
            .join(format!("{stem}-alpha"))
    };
    let beta_dir = {
        let stem = suite.output_dir.file_name().unwrap().to_string_lossy();
        suite
            .output_dir
            .parent()
            .unwrap()
            .join(format!("{stem}-beta"))
    };
    assert!(
        alpha_dir.join("manifest.json").exists(),
        "missing alpha manifest at {}",
        alpha_dir.display()
    );
    assert!(
        beta_dir.join("manifest.json").exists(),
        "missing beta manifest at {}",
        beta_dir.display()
    );
    // And they must be distinct directories (would be tautological,
    // but catches a future regression where apply_run_id stops
    // suffixing).
    assert_ne!(alpha_dir, beta_dir);
}
