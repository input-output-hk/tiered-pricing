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
    events::{Event, EventTracker},
    sim::Simulation,
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::{
    metrics::{
        MetricsCollector, RunSummary, comparison, diagnostics, time_series,
    },
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

/// Run a suite end-to-end. Builds the manifest if absent, executes
/// each (job, seed) skipping those already `Completed`, and writes
/// the per-suite `metrics_comparison.txt` at the end.
///
/// **Resume contract.** On re-run, completed jobs are skipped *and*
/// their `run_summary.json` artefacts are reloaded from disk before
/// writing `metrics_comparison.txt`, so the comparison file always
/// reflects the full suite (not just this invocation's jobs).
pub fn run_suite(suite_path: &Path) -> Result<()> {
    let suite = Suite::load(suite_path)?;
    let manifest_path = suite.output_dir.join("manifest.json");
    let mut manifest = Manifest::load_or_init(&manifest_path, &suite)?;
    // Persist initial state so a kill before any job runs leaves a
    // consistent manifest.
    manifest.save(&manifest_path)?;

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    let total_jobs = suite.job_seed_pairs().len();
    for (idx, (job_idx, seed)) in suite.job_seed_pairs().into_iter().enumerate() {
        let job = &suite.jobs[job_idx];
        let seed_key = seed.to_string();
        let entry = manifest
            .jobs
            .get(&job.name)
            .and_then(|m| m.get(&seed_key))
            .cloned()
            .unwrap_or(JobEntry {
                status: JobStatus::Pending,
                started_at_utc: None,
                completed_at_utc: None,
                output_path: None,
                error: None,
            });
        if entry.status == JobStatus::Completed {
            tracing::info!(
                "[{}/{}] skip (completed): {} seed={}",
                idx + 1,
                total_jobs,
                job.name,
                seed
            );
            continue;
        }
        tracing::info!(
            "[{}/{}] run: {} seed={}",
            idx + 1,
            total_jobs,
            job.name,
            seed
        );
        if let Some(jobs) = manifest.jobs.get_mut(&job.name) {
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
        manifest.save(&manifest_path)?;

        let result = runtime.block_on(async {
            run_job(&suite, job_idx, seed).await
        });

        let job_dir = suite
            .output_dir
            .join(&job.name)
            .join(seed.to_string());
        match result {
            Ok(summary) => {
                persist_run_artefacts(&job_dir, &summary)?;
                if let Some(jobs) = manifest.jobs.get_mut(&job.name) {
                    jobs.insert(
                        seed_key.clone(),
                        JobEntry {
                            status: JobStatus::Completed,
                            started_at_utc: entry.started_at_utc.or(Some(Utc::now())),
                            completed_at_utc: Some(Utc::now()),
                            output_path: Some(job_dir.clone()),
                            error: None,
                        },
                    );
                }
                tracing::info!(
                    "[{}/{}] done: {} seed={} included={} evicted={} hash={}",
                    idx + 1,
                    total_jobs,
                    job.name,
                    seed,
                    summary.total_txs_included,
                    summary.total_txs_evicted_quote_drift,
                    &summary.pricing_event_stream_sha256[..16]
                );
            }
            Err(e) => {
                if let Some(jobs) = manifest.jobs.get_mut(&job.name) {
                    jobs.insert(
                        seed_key.clone(),
                        JobEntry {
                            status: JobStatus::Failed,
                            started_at_utc: entry.started_at_utc,
                            completed_at_utc: Some(Utc::now()),
                            output_path: Some(job_dir.clone()),
                            error: Some(format!("{e:#}")),
                        },
                    );
                }
                manifest.save(&manifest_path)?;
                return Err(e).with_context(|| {
                    format!("job {} seed {} failed", job.name, seed)
                });
            }
        }
        manifest.save(&manifest_path)?;
    }

    // Per-suite metrics_comparison.txt — load every Completed job's
    // summary from disk so the comparison always reflects the full
    // suite (not just this invocation's jobs).
    let all_runs = collect_completed_runs(&suite, &manifest)?;
    let comparison_path = suite.output_dir.join("metrics_comparison.txt");
    comparison::write_suite(&comparison_path, &suite.suite_name, &all_runs)?;

    Ok(())
}

/// Persist `run_summary.json` and `pricing_event_stream.sha256` to
/// `<job_dir>` so a later resume can reload them.
fn persist_run_artefacts(job_dir: &Path, summary: &RunSummary) -> Result<()> {
    std::fs::create_dir_all(job_dir)?;
    std::fs::write(
        job_dir.join(RUN_SUMMARY_FILE),
        serde_json::to_string_pretty(summary)?,
    )?;
    std::fs::write(
        job_dir.join(HASH_FILE),
        &summary.pricing_event_stream_sha256,
    )?;
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
        let Some(entry) = manifest
            .jobs
            .get(&job.name)
            .and_then(|m| m.get(&seed_key))
        else {
            continue;
        };
        if entry.status != JobStatus::Completed {
            continue;
        }
        let job_dir = suite
            .output_dir
            .join(&job.name)
            .join(seed.to_string());
        let summary_path = job_dir.join(RUN_SUMMARY_FILE);
        let text = std::fs::read_to_string(&summary_path).with_context(|| {
            format!(
                "reading persisted run_summary at {} (manifest says completed)",
                summary_path.display()
            )
        })?;
        let summary: RunSummary = serde_json::from_str(&text).with_context(|| {
            format!("parsing run_summary at {}", summary_path.display())
        })?;
        out.push((job.name.clone(), seed, summary));
    }
    Ok(out)
}

/// Re-run every Completed (job, seed) of a suite and assert each
/// freshly-computed pricing-event-stream SHA256 matches the
/// persisted value. Used by `experiment-suite verify`.
pub fn verify_suite(suite_path: &Path) -> Result<()> {
    let suite = Suite::load(suite_path)?;
    let manifest_path = suite.output_dir.join("manifest.json");
    if !manifest_path.exists() {
        anyhow::bail!(
            "no manifest at {} — run the suite first",
            manifest_path.display()
        );
    }
    let manifest = Manifest::load_or_init(&manifest_path, &suite)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let mut mismatches = 0usize;
    let mut checked = 0usize;
    for (job_idx, seed) in suite.job_seed_pairs() {
        let job = &suite.jobs[job_idx];
        let seed_key = seed.to_string();
        let Some(entry) = manifest
            .jobs
            .get(&job.name)
            .and_then(|m| m.get(&seed_key))
        else {
            continue;
        };
        if entry.status != JobStatus::Completed {
            tracing::info!("skip (not completed): {} seed={}", job.name, seed);
            continue;
        }
        let job_dir = suite.output_dir.join(&job.name).join(seed.to_string());
        let stored = std::fs::read_to_string(job_dir.join(HASH_FILE))
            .with_context(|| {
                format!(
                    "reading {} (manifest says completed; expected hash file)",
                    job_dir.join(HASH_FILE).display()
                )
            })?;
        let stored = stored.trim();
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
        let summary = runtime.block_on(async { run_job(&suite, job_idx, seed).await })?;
        let fresh = summary.pricing_event_stream_sha256.trim();
        checked += 1;
        if stored == fresh {
            tracing::info!(
                "verify ok: {} seed={} hash={}",
                job.name,
                seed,
                &fresh[..16]
            );
        } else {
            mismatches += 1;
            tracing::error!(
                "verify FAIL: {} seed={} stored={} fresh={}",
                job.name,
                seed,
                stored,
                fresh
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
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("yaml");
    let merged = match ext {
        "toml" => figment.merge(Toml::file_exact(path)),
        _ => figment.merge(Yaml::file_exact(path)),
    };
    Ok(merged)
}

async fn run_job(suite: &Suite, job_idx: usize, seed: u64) -> Result<RunSummary> {
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
    let job_dir = suite
        .output_dir
        .join(&job.name)
        .join(seed.to_string());
    std::fs::create_dir_all(&job_dir)?;
    let time_series_path = job_dir.join("time_series.csv");
    let diagnostics_path = job_dir.join("diagnostics.log");

    // Build the metrics collector and pre-load the multiplier-floor
    // for the breach checker.
    let mut collector =
        MetricsCollector::new(config.block_generation_probability());
    if let sim_core::config::PricingConfig::TwoLane(s) = config.pricing_config() {
        collector.set_multiplier_floor(
            s.multiplier_floor.numerator,
            s.multiplier_floor.denominator,
        );
    }

    let (events_tx, mut events_rx) =
        mpsc::unbounded_channel::<(Event, sim_core::clock::Timestamp)>();
    let coordinator = ClockCoordinator::new(config.timestamp_resolution);
    let clock = coordinator.clock();
    let tracker = EventTracker::new(events_tx, clock.clone(), &config.nodes);

    // The simulation owns the only outstanding event-sender (via the
    // tracker). When `simulation.run` returns we drop it explicitly,
    // closing the channel and letting the drain task end.
    let drain = tokio::spawn(async move {
        while let Some((event, _ts)) = events_rx.recv().await {
            collector.ingest(&event);
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
    // Synthesise diagnostic notes from the resolved config + summary.
    let mut notes: Vec<diagnostics::DiagnosticNote> = Vec::new();
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
                message:
                    "RB-reserved variant: standard-fee txs are excluded from the RB body \
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
    diagnostics::write(&diagnostics_path, &config, &summary, &notes)?;
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `verify_suite` must bail when a Completed (job, seed) entry's
    /// persisted `pricing_event_stream.sha256` is empty or non-hex —
    /// otherwise an empty stored hash silently matches an empty
    /// freshly-computed hash and the determinism check passes by
    /// default. (See `runner.rs` lines 360-374.)
    #[test]
    fn verify_suite_bails_on_empty_stored_hash() {
        let tmp = tempfile::tempdir().unwrap();
        let suite_dir = tmp.path();
        let output_dir = suite_dir.join("output");
        let job_dir = output_dir.join("the_job").join("1");
        std::fs::create_dir_all(&job_dir).unwrap();
        // Empty hash file — the bug shape we're guarding against.
        std::fs::write(job_dir.join("pricing_event_stream.sha256"), "").unwrap();
        // run_summary.json must exist but the verifier never reads it
        // before the malformed-hash bail, so a stub is fine.
        std::fs::write(
            job_dir.join("run_summary.json"),
            r#"{"pricing_event_stream_sha256":"","total_txs_included":0,"total_txs_evicted_quote_drift":0,"multiplier_floor_breaches":0,"pricing_ticks":0,"components":[]}"#,
        )
        .unwrap();
        // Minimal suite YAML. Paths inside it never resolve because
        // run_job is unreachable past the bail.
        let suite_yaml = format!(
            "suite-name: t\noutput-dir: {}\nseeds: [1]\ndefault-slots: 1\n\
             default-topology: nope.yaml\ndefault-protocol: nope.yaml\n\
             default-demand: nope.yaml\njobs:\n  - name: the_job\n    pricing: nope.yaml\n",
            output_dir.display()
        );
        let suite_path = suite_dir.join("suite.yaml");
        std::fs::write(&suite_path, suite_yaml).unwrap();
        // Manifest with the (the_job, seed=1) entry marked Completed so
        // verify_suite tries to check its hash.
        let manifest = format!(
            r#"{{"suite-name":"t","started-at-utc":"2026-01-01T00:00:00Z",
                "jobs":{{"the_job":{{"1":{{"status":"completed",
                  "started-at-utc":"2026-01-01T00:00:00Z",
                  "completed-at-utc":"2026-01-01T00:00:00Z",
                  "output-path":"{}"}}}}}}}}"#,
            job_dir.display()
        );
        std::fs::write(output_dir.join("manifest.json"), manifest).unwrap();

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
        let suite_dir = tmp.path();
        let output_dir = suite_dir.join("output");
        let job_dir = output_dir.join("the_job").join("1");
        std::fs::create_dir_all(&job_dir).unwrap();
        // 64 chars of `z` — wrong length-wise it passes, but not hex.
        std::fs::write(job_dir.join("pricing_event_stream.sha256"), "z".repeat(64))
            .unwrap();
        std::fs::write(
            job_dir.join("run_summary.json"),
            r#"{"pricing_event_stream_sha256":"","total_txs_included":0,"total_txs_evicted_quote_drift":0,"multiplier_floor_breaches":0,"pricing_ticks":0,"components":[]}"#,
        )
        .unwrap();
        let suite_yaml = format!(
            "suite-name: t\noutput-dir: {}\nseeds: [1]\ndefault-slots: 1\n\
             default-topology: nope.yaml\ndefault-protocol: nope.yaml\n\
             default-demand: nope.yaml\njobs:\n  - name: the_job\n    pricing: nope.yaml\n",
            output_dir.display()
        );
        let suite_path = suite_dir.join("suite.yaml");
        std::fs::write(&suite_path, suite_yaml).unwrap();
        let manifest = format!(
            r#"{{"suite-name":"t","started-at-utc":"2026-01-01T00:00:00Z",
                "jobs":{{"the_job":{{"1":{{"status":"completed",
                  "started-at-utc":"2026-01-01T00:00:00Z",
                  "completed-at-utc":"2026-01-01T00:00:00Z",
                  "output-path":"{}"}}}}}}}}"#,
            job_dir.display()
        );
        std::fs::write(output_dir.join("manifest.json"), manifest).unwrap();

        let err = verify_suite(&suite_path).unwrap_err();
        assert!(format!("{err:#}").contains("malformed"));
    }
}
