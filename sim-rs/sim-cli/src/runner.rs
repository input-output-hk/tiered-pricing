//! Per-job simulator runner + resumable manifest. M3.
//!
//! Responsibilities:
//! - For each (job, seed) of a `Suite`: compose a `RawParameters`
//!   from the suite's protocol baseline + demand overlay + per-job
//!   pricing overlay + per-job optional overrides, build a
//!   `SimConfiguration`, run the simulation, and consume the event
//!   stream into a `MetricsCollector`.
//! - Write per-job `time_series.csv`, `diagnostics.log`, and a
//!   per-suite `metrics_comparison.txt`.
//! - Maintain a `manifest.json` recording per-(job, seed) status:
//!   `pending | running | completed | failed`. On resume, completed
//!   jobs are skipped; running/failed jobs are retried.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use figment::{
    Figment,
    providers::{Format, Toml, Yaml},
};
use serde::{Deserialize, Serialize};
use sim_core::{
    clock::ClockCoordinator,
    config::{RawParameters, RawTopology, SimConfiguration, Topology},
    events::{Event, EventFilter, EventTracker},
    sim::Simulation,
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::{
    metrics::{MetricsCollector, RunSummary, comparison, diagnostics, time_series},
    suite::Suite,
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct JobEntry {
    pub status: JobStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at_utc: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at_utc: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Manifest {
    pub suite_name: String,
    pub started_at_utc: DateTime<Utc>,
    /// `jobs[<job_name>][<seed>] = JobEntry`.
    pub jobs: BTreeMap<String, BTreeMap<String, JobEntry>>,
}

impl Manifest {
    pub fn load_or_init(path: &Path, suite: &Suite) -> Result<Self> {
        if path.exists() {
            let text = std::fs::read_to_string(path)?;
            let mut existing: Manifest = serde_json::from_str(&text)?;
            // Re-mark Running entries as Pending: a previous run was
            // killed mid-job, so we should retry.
            for jobs in existing.jobs.values_mut() {
                for entry in jobs.values_mut() {
                    if entry.status == JobStatus::Running {
                        entry.status = JobStatus::Pending;
                    }
                }
            }
            return Ok(existing);
        }
        let mut jobs: BTreeMap<String, BTreeMap<String, JobEntry>> = BTreeMap::new();
        for (idx, _) in suite.jobs.iter().enumerate() {
            let job = &suite.jobs[idx];
            let mut seed_map: BTreeMap<String, JobEntry> = BTreeMap::new();
            let seeds = job.overrides.seeds.as_ref().unwrap_or(&suite.seeds);
            for s in seeds {
                seed_map.insert(
                    s.to_string(),
                    JobEntry {
                        status: JobStatus::Pending,
                        started_at_utc: None,
                        completed_at_utc: None,
                        output_path: None,
                        error: None,
                    },
                );
            }
            jobs.insert(job.name.clone(), seed_map);
        }
        Ok(Self {
            suite_name: suite.suite_name.clone(),
            started_at_utc: Utc::now(),
            jobs,
        })
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text = serde_json::to_string_pretty(self)?;
        std::fs::write(path, text)?;
        Ok(())
    }
}

/// Filenames inside each (job, seed) directory.
const RUN_SUMMARY_FILE: &str = "run_summary.json";
const HASH_FILE: &str = "pricing_event_stream.sha256";
const PROGRESS_WRITE_EVERY_PRICING_TICKS: u64 = 50;
const PROGRESS_WRITE_EVERY_EVENTS: u64 = 50_000;
const PROGRESS_WRITE_WALL_INTERVAL: Duration = Duration::from_secs(10);

/// Run a suite end-to-end. Builds the manifest if absent, executes
/// each (job, seed) skipping those already `Completed`, and writes
/// the per-suite `metrics_comparison.txt` at the end.
///
/// **Resume contract.** On re-run, completed jobs are skipped *and*
/// their `run_summary.json` artefacts are reloaded from disk before
/// writing `metrics_comparison.txt`, so the comparison file always
/// reflects the full suite (not just this invocation's jobs).
/// Append `-<run_id>` to the suite's `output_dir` if `run_id` is set.
/// Used by `run_suite_with_run_id`, `verify_suite_with_run_id`, and
/// the `experiment-suite status` command so a single batch invocation
/// can timestamp all per-suite output dirs uniformly. No-op when
/// `run_id` is `None` (preserves legacy paths for unit tests and
/// one-off invocations).
pub fn apply_run_id(suite: &mut Suite, run_id: Option<&str>) {
    if let Some(id) = run_id {
        let parent = suite.output_dir.parent().map(PathBuf::from);
        let stem = suite
            .output_dir
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let suffixed = format!("{stem}-{id}");
        suite.output_dir = match parent {
            Some(p) => p.join(suffixed),
            None => PathBuf::from(suffixed),
        };
    }
}

pub fn run_suite(suite_path: &Path) -> Result<()> {
    run_suite_with_run_id(suite_path, None, 1)
}

/// Same as [`run_suite`], with an optional `run_id` that, if set,
/// appends `-<run_id>` to the suite's `output_dir`. Lets a single
/// invocation of the wrapper script timestamp all per-suite outputs
/// uniformly while preserving resume semantics when the same
/// `run_id` is passed again.
///
/// `parallelism` caps the number of concurrent (job, seed) pairs.
/// Per-(job, seed) determinism is the simulator's contract — `run_job`
/// produces bit-identical output regardless of how many siblings run
/// concurrently — so parallelism only changes wall-clock interleaving,
/// not the pricing event stream. Each parallel job owns its own
/// simulator state (config, topology, mempool, collector), so peak RSS
/// scales linearly in `parallelism`.
pub fn run_suite_with_run_id(
    suite_path: &Path,
    run_id: Option<&str>,
    parallelism: usize,
) -> Result<()> {
    let mut suite = Suite::load(suite_path)?;
    apply_run_id(&mut suite, run_id);
    let manifest_path = suite.output_dir.join("manifest.json");
    let manifest = Manifest::load_or_init(&manifest_path, &suite)?;
    // Persist initial state so a kill before any job runs leaves a
    // consistent manifest.
    manifest.save(&manifest_path)?;

    let parallelism = parallelism.max(1);
    let suite_arc = Arc::new(suite);
    let manifest_path_arc = Arc::new(manifest_path.clone());
    let total_jobs = suite_arc.job_seed_pairs().len();

    // Snapshot pending (job, seed) pairs BEFORE dispatch. Suite's natural
    // iteration order gives deterministic dispatch order; the manifest
    // snapshot is taken once so "is Completed" checks don't race with
    // our own writes inside the worker threads.
    let pending: Vec<(usize, usize, u64)> = suite_arc
        .job_seed_pairs()
        .into_iter()
        .enumerate()
        .filter(|(_, (job_idx, seed))| {
            let job = &suite_arc.jobs[*job_idx];
            let entry = manifest
                .jobs
                .get(&job.name)
                .and_then(|s| s.get(&seed.to_string()));
            !matches!(entry, Some(e) if e.status == JobStatus::Completed)
        })
        .map(|(seq_idx, (job_idx, seed))| (seq_idx, job_idx, seed))
        .collect();

    // Log already-Completed pairs in suite order so logs match the
    // pre-refactor sequential layout for the resumed-suite case.
    for (idx, (job_idx, seed)) in suite_arc.job_seed_pairs().into_iter().enumerate() {
        let job = &suite_arc.jobs[job_idx];
        let entry = manifest
            .jobs
            .get(&job.name)
            .and_then(|s| s.get(&seed.to_string()));
        if matches!(entry, Some(e) if e.status == JobStatus::Completed) {
            tracing::info!(
                "[{}/{}] skip (completed): {} seed={}",
                idx + 1,
                total_jobs,
                job.name,
                seed
            );
        }
    }

    // The `Simulation` future contains `Box<dyn Actor>` which is not
    // `Send` (sim-core/src/sim.rs:86), so we cannot drive it via a
    // multi-thread tokio runtime's `spawn`. Instead each parallel job
    // gets its own OS thread + per-thread `current_thread` runtime,
    // which keeps the simulation pinned to a single thread (no `Send`
    // required) while still parallelising across jobs. A
    // std::sync::Mutex around the manifest is sufficient because lock
    // hold-times are tiny (single fs::write + a metrics-comparison
    // O(completed) summary reload).
    let manifest_arc: Arc<Mutex<Manifest>> = Arc::new(Mutex::new(manifest));
    let (work_tx, work_rx) = std::sync::mpsc::channel::<(usize, usize, u64)>();
    let work_rx = Arc::new(Mutex::new(work_rx));

    // Outcome reported back by each worker so the driver can log and
    // aggregate failures consistently.
    struct JobOutcome {
        job_name: String,
        seed: u64,
        seq_idx: usize,
        result: Result<RunSummary>,
    }
    let (done_tx, done_rx) = std::sync::mpsc::channel::<JobOutcome>();

    for (seq_idx, job_idx, seed) in pending {
        work_tx.send((seq_idx, job_idx, seed)).unwrap();
    }
    drop(work_tx);

    let worker_count = parallelism;
    let mut workers = Vec::with_capacity(worker_count);
    for _ in 0..worker_count {
        let suite_w = Arc::clone(&suite_arc);
        let manifest_w = Arc::clone(&manifest_arc);
        let manifest_path_w = Arc::clone(&manifest_path_arc);
        let work_rx_w = Arc::clone(&work_rx);
        let done_tx_w = done_tx.clone();
        let handle = thread::spawn(move || -> Result<()> {
            // Per-thread current_thread runtime. Keeps the
            // !Send Simulation future pinned to this OS thread.
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            loop {
                let next = {
                    let rx = work_rx_w.lock().unwrap();
                    rx.recv()
                };
                let (seq_idx, job_idx, seed) = match next {
                    Ok(item) => item,
                    Err(_) => break, // channel closed, all work dispatched
                };
                let job_name = suite_w.jobs[job_idx].name.clone();
                let seed_key = seed.to_string();

                tracing::info!(
                    "[{}/{}] run: {} seed={}",
                    seq_idx + 1,
                    total_jobs,
                    job_name,
                    seed
                );

                // Manifest transition: Pending → Running. Hold the lock
                // for the read-modify-write window only. `Manifest::save`
                // does a single atomic `std::fs::write`, so on-disk state
                // is consistent at any SIGINT moment — the
                // `Manifest::load_or_init` recovery resets Running →
                // Pending on the next run.
                let prev_started_at = {
                    let mut m = manifest_w.lock().unwrap();
                    let prev = m
                        .jobs
                        .get(&job_name)
                        .and_then(|s| s.get(&seed_key))
                        .and_then(|e| e.started_at_utc);
                    if let Some(jobs) = m.jobs.get_mut(&job_name) {
                        jobs.insert(
                            seed_key.clone(),
                            JobEntry {
                                status: JobStatus::Running,
                                started_at_utc: Some(Utc::now()),
                                completed_at_utc: None,
                                output_path: None,
                                error: None,
                            },
                        );
                    }
                    m.save(&manifest_path_w)?;
                    prev
                };

                let result = runtime.block_on(async { run_job(&suite_w, job_idx, seed).await });
                let job_dir = suite_w.output_dir.join(&job_name).join(seed.to_string());

                // Persist artefacts before the manifest transition so a
                // crash between "Running" and "Completed" leaves the
                // artefacts on disk and the next run retries the job
                // (Running → Pending on reload), which is the safe
                // direction.
                let entry = match &result {
                    Ok(summary) => match persist_run_artefacts(&job_dir, summary) {
                        Err(e) => JobEntry {
                            status: JobStatus::Failed,
                            started_at_utc: prev_started_at.or(Some(Utc::now())),
                            completed_at_utc: Some(Utc::now()),
                            output_path: Some(job_dir.clone()),
                            error: Some(format!("persist_run_artefacts: {e:#}")),
                        },
                        Ok(()) => JobEntry {
                            status: JobStatus::Completed,
                            started_at_utc: prev_started_at.or(Some(Utc::now())),
                            completed_at_utc: Some(Utc::now()),
                            output_path: Some(job_dir.clone()),
                            error: None,
                        },
                    },
                    Err(e) => JobEntry {
                        status: JobStatus::Failed,
                        started_at_utc: prev_started_at,
                        completed_at_utc: Some(Utc::now()),
                        output_path: Some(job_dir.clone()),
                        error: Some(format!("{e:#}")),
                    },
                };

                // Manifest transition: Running → Completed | Failed.
                // Also rewrite metrics_comparison.txt under the same
                // lock so long-running suites have inspectable partial
                // comparison output. The metrics-comparison rebuild is
                // O(completed_jobs) JSON reads (~5-20 ms even for a
                // full suite), well under simulation runtime.
                {
                    let mut m = manifest_w.lock().unwrap();
                    if let Some(jobs) = m.jobs.get_mut(&job_name) {
                        jobs.insert(seed_key, entry);
                    }
                    m.save(&manifest_path_w)?;
                    write_suite_metrics_comparison(&suite_w, &m)?;
                }

                done_tx_w
                    .send(JobOutcome {
                        job_name,
                        seed,
                        seq_idx,
                        result,
                    })
                    .ok();
            }
            Ok(())
        });
        workers.push(handle);
    }
    drop(done_tx);

    // Aggregate-and-continue rather than fail-fast: a failed (job, seed)
    // ends Failed in the manifest (recoverable on next run), and
    // cancelling siblings on first failure would waste in-flight compute.
    // CI callers that want fail-fast can rely on the non-zero exit code
    // that propagates from the final `bail!`.
    let mut failures: Vec<anyhow::Error> = Vec::new();
    while let Ok(outcome) = done_rx.recv() {
        match outcome.result {
            Ok(summary) => tracing::info!(
                "[{}/{}] done: {} seed={} included={} evicted={} hash={}",
                outcome.seq_idx + 1,
                total_jobs,
                outcome.job_name,
                outcome.seed,
                summary.total_txs_included,
                summary.total_txs_evicted_quote_drift,
                &summary.pricing_event_stream_sha256[..16]
            ),
            Err(e) => {
                tracing::error!(
                    "[{}/{}] FAILED: {} seed={}: {e:#}",
                    outcome.seq_idx + 1,
                    total_jobs,
                    outcome.job_name,
                    outcome.seed
                );
                failures.push(e.context(format!(
                    "job {} seed {} failed",
                    outcome.job_name, outcome.seed
                )));
            }
        }
    }

    // Surface worker-thread infrastructure errors (manifest save
    // failures, runtime build failures, etc.) as suite failures.
    for handle in workers {
        match handle.join() {
            Ok(Ok(())) => {}
            Ok(Err(e)) => failures.push(e.context("worker thread reported an error")),
            Err(_) => failures.push(anyhow::anyhow!("worker thread panicked")),
        }
    }

    if !failures.is_empty() {
        let mut combined = anyhow::anyhow!("{} (job, seed) pair(s) failed", failures.len());
        for e in failures {
            combined = combined.context(format!("{e:#}"));
        }
        return Err(combined);
    }

    // Final suite-end metrics_comparison rewrite. The in-loop writer
    // also runs after the last completion under the manifest mutex, so
    // this trailing write is mostly for the case where no jobs ran
    // (all already-Completed on resume) — gives the user a fresh
    // comparison file regardless.
    let manifest_final = manifest_arc.lock().unwrap();
    write_suite_metrics_comparison(&suite_arc, &manifest_final)?;

    Ok(())
}

fn write_suite_metrics_comparison(suite: &Suite, manifest: &Manifest) -> Result<()> {
    // Per-suite metrics_comparison.txt — load every Completed job's
    // summary from disk so the comparison always reflects the full
    // suite (not just this invocation's jobs). This is called after
    // each completed job as well as at suite end so long-running
    // suites leave inspectable partial comparison output.
    let all_runs = collect_completed_runs(suite, manifest)?;
    let comparison_path = suite.output_dir.join("metrics_comparison.txt");
    comparison::write_suite(&comparison_path, &suite.suite_name, &all_runs)?;
    Ok(())
}

/// Persist `run_summary.json` and `pricing_event_stream.sha256` to
/// `<job_dir>` so a later resume can reload them.
fn persist_run_artefacts(job_dir: &Path, summary: &RunSummary) -> Result<()> {
    std::fs::create_dir_all(job_dir)?;
    persist_run_summary(job_dir, summary)?;
    std::fs::write(
        job_dir.join(HASH_FILE),
        &summary.pricing_event_stream_sha256,
    )?;
    Ok(())
}

fn persist_run_summary(job_dir: &Path, summary: &RunSummary) -> Result<()> {
    std::fs::create_dir_all(job_dir)?;
    std::fs::write(
        job_dir.join(RUN_SUMMARY_FILE),
        serde_json::to_string_pretty(summary)?,
    )?;
    Ok(())
}

fn diagnostic_notes(
    config: &SimConfiguration,
    summary: &RunSummary,
    in_progress: bool,
) -> Vec<diagnostics::DiagnosticNote> {
    let mut notes: Vec<diagnostics::DiagnosticNote> = Vec::new();
    if in_progress {
        notes.push(diagnostics::DiagnosticNote {
            level: diagnostics::NoteLevel::Info,
            message:
                "run is still in progress; run summary, time series, diagnostics, and pricing hash are partial"
                    .to_string(),
        });
    }
    if let sim_core::config::PricingConfig::TwoLane(s) = config.pricing_config() {
        if matches!(
            s.variant,
            sim_core::tx_pricing::TwoLaneVariant::RbReservedPriorityOnly
                | sim_core::tx_pricing::TwoLaneVariant::RbReservedBothDynamic
        ) {
            // Plan line 320 asks for an RB-reserved rejection count
            // in diagnostics.log. Standard-fee txs are skipped by
            // the validity rule during RB-body packing
            // (sample_from_mempool_lane_aware) — not by an event —
            // so the rejection isn't directly observable from the
            // event stream. Point the reader at the CSV column that
            // carries the equivalent evidence.
            notes.push(diagnostics::DiagnosticNote {
                level: diagnostics::NoteLevel::Info,
                message: "RB-reserved variant: standard-fee txs are excluded from the RB body \
                     by the validity rule (implementation-plan.md line 91). The CSV \
                     column `included_count_standard` records the count of standard-fee \
                     txs that landed on chain; under RB-reserved variants this column \
                     should be 0 except where an EB partition refunded a priority-fee \
                     tx down to standard."
                    .to_string(),
            });
        }
    }
    if summary.multiplier_floor_breaches > 0 {
        notes.push(diagnostics::DiagnosticNote {
            level: diagnostics::NoteLevel::Error,
            message: format!(
                "multiplier-floor invariant breached {} time(s); spec invariant requires 0",
                summary.multiplier_floor_breaches
            ),
        });
    }
    notes
}

fn write_progress_artefacts(
    job_dir: &Path,
    time_series_path: &Path,
    diagnostics_path: &Path,
    config: &SimConfiguration,
    collector: &MetricsCollector,
) -> Result<()> {
    let (rows, summary) = collector.snapshot();
    persist_run_summary(job_dir, &summary)?;
    time_series::write_csv(time_series_path, &rows)?;
    let notes = diagnostic_notes(config, &summary, true);
    diagnostics::write(diagnostics_path, config, &summary, &notes)?;
    Ok(())
}

/// Walk the manifest and load every Completed job's persisted
/// `run_summary.json`. Returns `(job_name, seed, summary)` tuples in
/// the suite's natural (job × seed) iteration order.
fn collect_completed_runs(
    suite: &Suite,
    manifest: &Manifest,
) -> Result<Vec<(String, u64, RunSummary)>> {
    let mut out = Vec::new();
    for (job_idx, seed) in suite.job_seed_pairs() {
        let job = &suite.jobs[job_idx];
        let seed_key = seed.to_string();
        let Some(entry) = manifest.jobs.get(&job.name).and_then(|m| m.get(&seed_key)) else {
            continue;
        };
        if entry.status != JobStatus::Completed {
            continue;
        }
        let job_dir = suite.output_dir.join(&job.name).join(seed.to_string());
        let summary_path = job_dir.join(RUN_SUMMARY_FILE);
        let text = std::fs::read_to_string(&summary_path).with_context(|| {
            format!(
                "reading persisted run_summary at {} (manifest says completed)",
                summary_path.display()
            )
        })?;
        let summary: RunSummary = serde_json::from_str(&text)
            .with_context(|| format!("parsing run_summary at {}", summary_path.display()))?;
        out.push((job.name.clone(), seed, summary));
    }
    Ok(out)
}

/// Re-run every Completed (job, seed) of a suite and assert each
/// freshly-computed pricing-event-stream SHA256 matches the
/// persisted value. Used by `experiment-suite verify`.
pub fn verify_suite(suite_path: &Path) -> Result<()> {
    verify_suite_with_run_id(suite_path, None, 1)
}

/// See [`run_suite_with_run_id`] for the `run_id` and `parallelism`
/// semantics. Verify is read-only over the manifest (no transitions)
/// so no mutex is needed; concurrent tasks each run their (job, seed)
/// independently and the driver aggregates outcomes after the JoinSet
/// drains.
pub fn verify_suite_with_run_id(
    suite_path: &Path,
    run_id: Option<&str>,
    parallelism: usize,
) -> Result<()> {
    let mut suite = Suite::load(suite_path)?;
    apply_run_id(&mut suite, run_id);
    let manifest_path = suite.output_dir.join("manifest.json");
    if !manifest_path.exists() {
        anyhow::bail!(
            "no manifest at {} — run the suite first",
            manifest_path.display()
        );
    }
    let manifest = Manifest::load_or_init(&manifest_path, &suite)?;
    let parallelism = parallelism.max(1);

    // Outcome of a single (job, seed) verify task.
    struct VerifyOutcome {
        job_name: String,
        seed: u64,
        matched: bool,
        stored: String,
        fresh: String,
    }

    // Pre-flight: walk the manifest in suite order, decide what to
    // check, surface any malformed hashes BEFORE spawning any work.
    // Same defensive bail as the previous serial implementation
    // (corrupt/hand-edited summaries deserialise as empty hashes;
    // silent pass-by-default is worse than aborting). Doing this
    // before spawning is also a micro-optimisation: we don't waste
    // simulator cycles on jobs whose stored hash is unrecoverable.
    let suite_arc = Arc::new(suite);
    let mut to_verify: Vec<(usize, u64, String)> = Vec::new();
    for (job_idx, seed) in suite_arc.job_seed_pairs() {
        let job = &suite_arc.jobs[job_idx];
        let seed_key = seed.to_string();
        let Some(entry) = manifest.jobs.get(&job.name).and_then(|m| m.get(&seed_key)) else {
            continue;
        };
        if entry.status != JobStatus::Completed {
            tracing::info!("skip (not completed): {} seed={}", job.name, seed);
            continue;
        }
        let job_dir = suite_arc.output_dir.join(&job.name).join(seed.to_string());
        let stored = std::fs::read_to_string(job_dir.join(HASH_FILE)).with_context(|| {
            format!(
                "reading {} (manifest says completed; expected hash file)",
                job_dir.join(HASH_FILE).display()
            )
        })?;
        let stored = stored.trim().to_string();
        // Defensive: a corrupt or hand-edited summary file with a
        // missing `pricing_event_stream_sha256` deserialises with an
        // empty string under `#[serde(default)]`, and the dedicated
        // `pricing_event_stream.sha256` file would be empty too.
        // Reject obviously-malformed values so we don't silently
        // pass-by-default against a freshly-computed empty-stream
        // hash.
        if stored.len() != 64 || !stored.chars().all(|c| c.is_ascii_hexdigit()) {
            anyhow::bail!(
                "stored hash at {} is malformed (expected 64 hex chars, got {:?}) — \
                 re-run the suite to regenerate",
                job_dir.join(HASH_FILE).display(),
                stored,
            );
        }
        to_verify.push((job_idx, seed, stored));
    }

    // Worker-thread pool: same shape as `run_suite_with_run_id` — the
    // `Simulation` future isn't `Send` so we drive it via per-thread
    // `current_thread` runtimes rather than a tokio multi-thread pool.
    // No manifest mutation during verify, so no shared lock needed.
    let (work_tx, work_rx) = std::sync::mpsc::channel::<(usize, u64, String)>();
    let work_rx = Arc::new(Mutex::new(work_rx));
    let (done_tx, done_rx) = std::sync::mpsc::channel::<Result<VerifyOutcome>>();

    for item in to_verify.into_iter() {
        work_tx.send(item).unwrap();
    }
    drop(work_tx);

    let worker_count = parallelism;
    let mut workers = Vec::with_capacity(worker_count);
    for _ in 0..worker_count {
        let suite_w = Arc::clone(&suite_arc);
        let work_rx_w = Arc::clone(&work_rx);
        let done_tx_w = done_tx.clone();
        let handle = thread::spawn(move || -> Result<()> {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            loop {
                let next = {
                    let rx = work_rx_w.lock().unwrap();
                    rx.recv()
                };
                let (job_idx, seed, stored) = match next {
                    Ok(item) => item,
                    Err(_) => break,
                };
                let job_name = suite_w.jobs[job_idx].name.clone();
                let res = runtime
                    .block_on(async { run_job(&suite_w, job_idx, seed).await })
                    .with_context(|| format!("re-running {job_name} seed={seed} for verify"));
                let outcome = res.map(|summary| {
                    let fresh = summary.pricing_event_stream_sha256.trim().to_string();
                    let matched = fresh == stored;
                    VerifyOutcome {
                        job_name: job_name.clone(),
                        seed,
                        matched,
                        stored,
                        fresh,
                    }
                });
                done_tx_w.send(outcome).ok();
            }
            Ok(())
        });
        workers.push(handle);
    }
    drop(done_tx);

    // Aggregate-and-continue, same rationale as `run_suite`: don't
    // cancel siblings on first failure, so the user sees the full
    // verify report. Per-task errors (e.g. config build failure during
    // re-run) collect alongside outcomes; we surface them after all
    // tasks finish.
    let mut outcomes: Vec<VerifyOutcome> = Vec::new();
    let mut errors: Vec<anyhow::Error> = Vec::new();
    while let Ok(item) = done_rx.recv() {
        match item {
            Ok(o) => outcomes.push(o),
            Err(e) => errors.push(e),
        }
    }
    for handle in workers {
        match handle.join() {
            Ok(Ok(())) => {}
            Ok(Err(e)) => errors.push(e.context("verify worker thread reported an error")),
            Err(_) => errors.push(anyhow::anyhow!("verify worker thread panicked")),
        }
    }
    if !errors.is_empty() {
        let mut combined = anyhow::anyhow!("{} verify task(s) errored", errors.len());
        for e in errors {
            combined = combined.context(format!("{e:#}"));
        }
        return Err(combined);
    }

    // Log in deterministic suite order regardless of completion order
    // so users (and any future log-text diffs) see the same lines either
    // way.
    let pair_order: BTreeMap<(String, u64), usize> = suite_arc
        .job_seed_pairs()
        .into_iter()
        .enumerate()
        .map(|(i, (job_idx, seed))| ((suite_arc.jobs[job_idx].name.clone(), seed), i))
        .collect();
    outcomes.sort_by_key(|o| pair_order.get(&(o.job_name.clone(), o.seed)).copied());

    let checked = outcomes.len();
    let mut mismatches = 0usize;
    for o in &outcomes {
        if o.matched {
            tracing::info!(
                "verify ok: {} seed={} hash={}",
                o.job_name,
                o.seed,
                &o.fresh[..16]
            );
        } else {
            mismatches += 1;
            tracing::error!(
                "verify FAIL: {} seed={} stored={} fresh={}",
                o.job_name,
                o.seed,
                o.stored,
                o.fresh
            );
        }
    }

    if mismatches > 0 {
        anyhow::bail!(
            "determinism verify failed: {} of {} (job, seed) pairs produced a different hash",
            mismatches,
            checked
        );
    }
    tracing::info!("determinism verify ok: {checked} (job, seed) pairs match");
    Ok(())
}

fn merge_layer(figment: Figment, path: &Path) -> Result<Figment> {
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("yaml");
    let merged = match ext {
        "toml" => figment.merge(Toml::file_exact(path)),
        _ => figment.merge(Yaml::file_exact(path)),
    };
    Ok(merged)
}

pub async fn run_job(suite: &Suite, job_idx: usize, seed: u64) -> Result<RunSummary> {
    let job = &suite.jobs[job_idx];
    let topology_path = job
        .overrides
        .topology
        .clone()
        .unwrap_or_else(|| suite.default_topology.clone());
    let protocol_path = job
        .overrides
        .protocol
        .clone()
        .unwrap_or_else(|| suite.default_protocol.clone());
    let demand_path = job
        .overrides
        .demand
        .clone()
        .unwrap_or_else(|| suite.default_demand.clone());
    let slots = job.overrides.slots.unwrap_or(suite.default_slots);

    let topology_text = std::fs::read_to_string(&topology_path)
        .with_context(|| format!("reading topology {}", topology_path.display()))?;
    let raw_topology: RawTopology = serde_yaml::from_str(&topology_text)?;
    let topology: Topology = raw_topology.into();
    topology.validate()?;

    // Compose the RawParameters by layering:
    //   1. the embedded `config.default.yaml` (provides every
    //      required field with sensible defaults),
    //   2. the suite's protocol-base overlay (phase-2 specifics),
    //   3. the demand profile (`actors:` block),
    //   4. the per-job pricing overlay.
    // File format is detected by extension (.yaml/.yml or .toml);
    // the embedded base is always YAML.
    let base = Figment::new().merge(Yaml::string(include_str!(
        "../../parameters/config.default.yaml"
    )));
    let raw = merge_layer(base, &protocol_path)?;
    let raw = merge_layer(raw, &demand_path)?;
    let raw = merge_layer(raw, &job.pricing)?;
    let params: RawParameters = raw
        .extract()
        .with_context(|| format!("composing params for job {}", job.name))?;
    let mut config = SimConfiguration::build(params, topology)?;
    config.seed = seed;
    config.slots = Some(slots);

    // Output paths.
    let job_dir = suite.output_dir.join(&job.name).join(seed.to_string());
    std::fs::create_dir_all(&job_dir)?;
    let time_series_path = job_dir.join("time_series.csv");
    let diagnostics_path = job_dir.join("diagnostics.log");

    // Build the metrics collector and pre-load the multiplier-floor
    // for the breach checker.
    let mut collector = MetricsCollector::new(config.block_generation_probability());
    if let sim_core::config::PricingConfig::TwoLane(s) = config.pricing_config() {
        collector
            .set_multiplier_floor(s.multiplier_floor.numerator, s.multiplier_floor.denominator);
    }
    collector.set_shock_window_slots(config.endorsement_window_slots());
    // Pin the time-series representative to the lexicographically
    // smallest node name. Without this, the first node to schedule
    // its `PricingTick` task wins, which depends on tokio scheduling
    // rather than the simulator seed.
    if let Some(name) = config.nodes.iter().map(|n| &n.name).min() {
        collector.set_representative_node(name.clone());
    }
    write_progress_artefacts(
        &job_dir,
        &time_series_path,
        &diagnostics_path,
        &config,
        &collector,
    )?;

    let (events_tx, mut events_rx) =
        mpsc::unbounded_channel::<(Event, sim_core::clock::Timestamp)>();
    let coordinator = ClockCoordinator::new(config.timestamp_resolution);
    let clock = coordinator.clock();
    let tracker = EventTracker::new_filtered(
        events_tx,
        clock.clone(),
        &config.nodes,
        EventFilter::Metrics,
    );

    // The simulation owns the only outstanding event-sender (via the
    // tracker). When `simulation.run` returns we drop it explicitly,
    // closing the channel and letting the drain task end.
    let progress_job_dir = job_dir.clone();
    let progress_time_series_path = time_series_path.clone();
    let progress_diagnostics_path = diagnostics_path.clone();
    let progress_config = config.clone();
    let drain = tokio::spawn(async move {
        let mut events_seen = 0u64;
        let mut last_progress_events = 0u64;
        let mut last_progress_ticks = collector.pricing_ticks();
        let mut last_progress_at = Instant::now();
        while let Some((event, _ts)) = events_rx.recv().await {
            collector.ingest(&event);
            events_seen = events_seen.saturating_add(1);
            let ticks = collector.pricing_ticks();
            let should_write = ticks
                >= last_progress_ticks.saturating_add(PROGRESS_WRITE_EVERY_PRICING_TICKS)
                || events_seen >= last_progress_events.saturating_add(PROGRESS_WRITE_EVERY_EVENTS)
                || last_progress_at.elapsed() >= PROGRESS_WRITE_WALL_INTERVAL;
            if should_write {
                if let Err(err) = write_progress_artefacts(
                    &progress_job_dir,
                    &progress_time_series_path,
                    &progress_diagnostics_path,
                    &progress_config,
                    &collector,
                ) {
                    tracing::warn!(
                        "failed to write in-progress metrics for {}: {err:#}",
                        progress_job_dir.display()
                    );
                }
                last_progress_ticks = ticks;
                last_progress_events = events_seen;
                last_progress_at = Instant::now();
            }
        }
        if let Err(err) = write_progress_artefacts(
            &progress_job_dir,
            &progress_time_series_path,
            &progress_diagnostics_path,
            &progress_config,
            &collector,
        ) {
            tracing::warn!(
                "failed to write final in-progress metrics for {}: {err:#}",
                progress_job_dir.display()
            );
        }
        collector
    });

    let mut simulation = Simulation::new(config.clone(), tracker, coordinator).await?;
    let token = CancellationToken::new();
    simulation.run(token).await?;
    // Drop simulation → drops EventTracker → closes the channel.
    drop(simulation);
    let collector = drain.await?;
    let (rows, summary) = collector.finalise();
    time_series::write_csv(&time_series_path, &rows)?;
    persist_run_summary(&job_dir, &summary)?;
    let notes = diagnostic_notes(&config, &summary, false);
    diagnostics::write(&diagnostics_path, &config, &summary, &notes)?;
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Common scaffolding: lays down a tempdir-backed
    /// (suite.yaml, manifest.json, run_summary.json) so the verifier
    /// reaches the malformed-hash bail. The hash file's contents are
    /// what the test varies. Returns the suite path the verifier
    /// should be pointed at.
    fn lay_down_verify_suite_fixture(tmp: &tempfile::TempDir, hash_contents: &str) -> PathBuf {
        let suite_dir = tmp.path();
        let output_dir = suite_dir.join("output");
        let job_dir = output_dir.join("the_job").join("1");
        std::fs::create_dir_all(&job_dir).unwrap();
        std::fs::write(job_dir.join("pricing_event_stream.sha256"), hash_contents).unwrap();
        // The verifier never reads `run_summary.json` before the
        // malformed-hash bail; a stub matching the on-disk schema is
        // sufficient. Use `serde_json::json!` rather than a raw
        // string literal so a future field rename in `RunSummary`
        // doesn't silently let this drift.
        let run_summary = serde_json::json!({
            "pricing_event_stream_sha256": "",
            "total_txs_included": 0,
            "total_txs_evicted_quote_drift": 0,
            "multiplier_floor_breaches": 0,
            "pricing_ticks": 0,
            "components": [],
        });
        std::fs::write(
            job_dir.join("run_summary.json"),
            serde_json::to_string(&run_summary).unwrap(),
        )
        .unwrap();
        // Suite YAML: paths inside never resolve because run_job is
        // unreachable past the bail.
        let suite_yaml = format!(
            "suite-name: t\noutput-dir: {}\nseeds: [1]\ndefault-slots: 1\n\
             default-topology: nope.yaml\ndefault-protocol: nope.yaml\n\
             default-demand: nope.yaml\njobs:\n  - name: the_job\n    pricing: nope.yaml\n",
            output_dir.display()
        );
        let suite_path = suite_dir.join("suite.yaml");
        std::fs::write(&suite_path, suite_yaml).unwrap();
        // Manifest marking (the_job, seed=1) Completed so verify_suite
        // tries to check its hash. Built via `serde_json::json!` so a
        // future Manifest/JobEntry rename surfaces here as a
        // serialise-side error rather than a silent string mismatch.
        let manifest = serde_json::json!({
            "suite-name": "t",
            "started-at-utc": "2026-01-01T00:00:00Z",
            "jobs": {
                "the_job": {
                    "1": {
                        "status": "completed",
                        "started-at-utc": "2026-01-01T00:00:00Z",
                        "completed-at-utc": "2026-01-01T00:00:00Z",
                        "output-path": job_dir.to_string_lossy(),
                    }
                }
            }
        });
        std::fs::write(
            output_dir.join("manifest.json"),
            serde_json::to_string(&manifest).unwrap(),
        )
        .unwrap();
        suite_path
    }

    /// `verify_suite` must bail when a Completed (job, seed) entry's
    /// persisted `pricing_event_stream.sha256` is empty or non-hex —
    /// otherwise an empty stored hash silently matches an empty
    /// freshly-computed hash and the determinism check passes by
    /// default. (See `runner.rs` lines 360-374.)
    #[test]
    fn verify_suite_bails_on_empty_stored_hash() {
        let tmp = tempfile::tempdir().unwrap();
        let suite_path = lay_down_verify_suite_fixture(&tmp, "");
        let err = verify_suite(&suite_path).unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("malformed"),
            "expected error message to contain 'malformed', got: {msg}"
        );
    }

    /// Sanity: a 64-char-but-non-hex stored value also bails. Catches a
    /// regression where someone relaxes the check to `len() == 64` only.
    #[test]
    fn verify_suite_bails_on_non_hex_stored_hash() {
        let tmp = tempfile::tempdir().unwrap();
        let suite_path = lay_down_verify_suite_fixture(&tmp, &"z".repeat(64));
        let err = verify_suite(&suite_path).unwrap_err();
        assert!(format!("{err:#}").contains("malformed"));
    }
}
