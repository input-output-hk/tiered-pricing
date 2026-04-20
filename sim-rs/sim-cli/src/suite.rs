use std::{
    collections::BTreeSet,
    fs,
    future::Future,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use tokio_util::sync::CancellationToken;

use crate::runner::{self, RunOutcome, RunRequest};

const DEFAULT_SUITE_OUTPUT_ROOT: &str = "output/experiment-suites";
const MANIFEST_FILE: &str = "manifest.json";
const SUITE_COPY_FILE: &str = "suite.yaml";

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct SuiteConfig {
    #[serde(default)]
    defaults: SuiteDefaults,
    #[serde(default)]
    jobs: Vec<SuiteJobSpec>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct SuiteDefaults {
    topology: Option<PathBuf>,
    #[serde(default)]
    parameters: Vec<PathBuf>,
    slots: Option<u64>,
    timescale: Option<f64>,
    trace: Option<bool>,
    #[serde(default)]
    trace_nodes: Vec<usize>,
    aggregate_events: Option<bool>,
    conformance_events: Option<bool>,
    output_root: Option<PathBuf>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct SuiteJobSpec {
    id: String,
    label: Option<String>,
    topology: Option<PathBuf>,
    #[serde(default)]
    parameters: Vec<PathBuf>,
    #[serde(default)]
    compare_parameters: Vec<PathBuf>,
    slots: Option<u64>,
    timescale: Option<f64>,
    trace: Option<bool>,
    #[serde(default)]
    trace_nodes: Vec<usize>,
    aggregate_events: Option<bool>,
    conformance_events: Option<bool>,
    #[serde(default)]
    seeds: Vec<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum SuiteStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Interrupted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Interrupted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct SuiteManifest {
    version: u32,
    label: String,
    status: SuiteStatus,
    source_suite: PathBuf,
    suite_copy: PathBuf,
    output_root: PathBuf,
    created_at: String,
    updated_at: String,
    jobs: Vec<ManifestJob>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct ManifestJob {
    expanded_id: String,
    job_id: String,
    label: String,
    seed: Option<u64>,
    status: JobStatus,
    attempt_count: u32,
    attempt_root: PathBuf,
    run: ManifestJobRun,
    attempts: Vec<JobAttempt>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct ManifestJobRun {
    topology: Option<PathBuf>,
    parameters: Vec<PathBuf>,
    compare_parameters: Vec<PathBuf>,
    slots: Option<u64>,
    timescale: Option<f64>,
    trace: bool,
    trace_nodes: Vec<usize>,
    aggregate_events: bool,
    conformance_events: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct JobAttempt {
    index: u32,
    status: JobStatus,
    dir: PathBuf,
    started_at: String,
    finished_at: Option<String>,
    error: Option<String>,
    report_path: Option<PathBuf>,
    case_outputs: Vec<JobAttemptCaseOutput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct JobAttemptCaseOutput {
    label: String,
    output_path: Option<PathBuf>,
}

pub async fn run_suite(
    suite_path: impl AsRef<Path>,
    label_override: Option<&str>,
    output_root_override: Option<&Path>,
    shutdown: CancellationToken,
) -> Result<PathBuf> {
    run_suite_with_runner(
        suite_path,
        label_override,
        output_root_override,
        shutdown,
        &|request, token| async move { runner::execute_run_request(&request, token).await },
    )
    .await
}

pub async fn resume_suite(
    suite_run_dir: impl AsRef<Path>,
    shutdown: CancellationToken,
) -> Result<()> {
    resume_suite_with_runner(suite_run_dir, shutdown, &|request, token| async move {
        runner::execute_run_request(&request, token).await
    })
    .await
}

async fn run_suite_with_runner<F, Fut>(
    suite_path: impl AsRef<Path>,
    label_override: Option<&str>,
    output_root_override: Option<&Path>,
    shutdown: CancellationToken,
    run_request_fn: &F,
) -> Result<PathBuf>
where
    F: Fn(RunRequest, CancellationToken) -> Fut,
    Fut: Future<Output = Result<RunOutcome>>,
{
    let suite_path = suite_path.as_ref();
    let config = load_suite_config(suite_path)?;
    let output_root = output_root_override
        .map(PathBuf::from)
        .or_else(|| config.defaults.output_root.clone())
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SUITE_OUTPUT_ROOT));
    fs::create_dir_all(&output_root)?;

    let label = resolve_suite_label(suite_path, label_override)?;
    let run_dir = allocate_run_dir(&output_root, &label)?;
    fs::create_dir_all(&run_dir)?;
    fs::copy(suite_path, run_dir.join(SUITE_COPY_FILE)).with_context(|| {
        format!(
            "failed to copy suite config {} into {}",
            suite_path.display(),
            run_dir.display()
        )
    })?;

    let created_at = now_rfc3339()?;
    let mut manifest = SuiteManifest {
        version: 1,
        label,
        status: SuiteStatus::Pending,
        source_suite: suite_path.to_path_buf(),
        suite_copy: PathBuf::from(SUITE_COPY_FILE),
        output_root,
        created_at: created_at.clone(),
        updated_at: created_at,
        jobs: expand_jobs(&config)?,
    };
    write_manifest(&run_dir, &mut manifest)?;

    execute_manifest_jobs(&run_dir, &mut manifest, shutdown, run_request_fn).await?;
    Ok(run_dir)
}

async fn resume_suite_with_runner<F, Fut>(
    suite_run_dir: impl AsRef<Path>,
    shutdown: CancellationToken,
    run_request_fn: &F,
) -> Result<()>
where
    F: Fn(RunRequest, CancellationToken) -> Fut,
    Fut: Future<Output = Result<RunOutcome>>,
{
    let run_dir = suite_run_dir.as_ref();
    let mut manifest = load_manifest(run_dir)?;
    if mark_stale_running_jobs(&mut manifest)? {
        write_manifest(run_dir, &mut manifest)?;
    }
    execute_manifest_jobs(run_dir, &mut manifest, shutdown, run_request_fn).await
}

fn load_suite_config(path: &Path) -> Result<SuiteConfig> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("failed to read suite config {}", path.display()))?;
    let config: SuiteConfig = serde_yaml::from_str(&text)
        .with_context(|| format!("failed to parse suite config {}", path.display()))?;
    validate_suite_config(&config)?;
    Ok(config)
}

fn validate_suite_config(config: &SuiteConfig) -> Result<()> {
    if config.jobs.is_empty() {
        bail!("suite config must define at least one job");
    }

    if let Some(timescale) = config.defaults.timescale
        && (!timescale.is_finite() || timescale <= 0.0)
    {
        bail!("defaults.timescale must be a positive finite number");
    }

    let mut seen_ids = BTreeSet::new();
    for job in &config.jobs {
        validate_job_id(&job.id)?;
        if !seen_ids.insert(job.id.clone()) {
            bail!("duplicate suite job id '{}'", job.id);
        }
        if let Some(timescale) = job.timescale
            && (!timescale.is_finite() || timescale <= 0.0)
        {
            bail!(
                "job '{}' timescale must be a positive finite number",
                job.id
            );
        }
        let unique_seed_count = job.seeds.iter().copied().collect::<BTreeSet<_>>().len();
        if unique_seed_count != job.seeds.len() {
            bail!("job '{}' defines duplicate seeds", job.id);
        }
    }

    Ok(())
}

fn validate_job_id(id: &str) -> Result<()> {
    if id.is_empty() {
        bail!("suite job id cannot be empty");
    }
    if !id
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
    {
        bail!(
            "suite job id '{}' may only contain ASCII letters, digits, '_' and '-'",
            id
        );
    }
    Ok(())
}

fn expand_jobs(config: &SuiteConfig) -> Result<Vec<ManifestJob>> {
    let mut jobs = Vec::new();

    for job in &config.jobs {
        let mut trace_nodes = config.defaults.trace_nodes.clone();
        trace_nodes.extend(job.trace_nodes.iter().copied());
        let trace_nodes: Vec<usize> = trace_nodes
            .into_iter()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();

        let mut parameters = config.defaults.parameters.clone();
        parameters.extend(job.parameters.iter().cloned());

        let seeds = if job.seeds.is_empty() {
            vec![None]
        } else {
            job.seeds.iter().copied().map(Some).collect()
        };

        for seed in seeds {
            let expanded_id = expanded_job_id(&job.id, seed);
            let attempt_root = match seed {
                Some(seed) => PathBuf::from("jobs")
                    .join(&job.id)
                    .join(format!("seed-{seed}")),
                None => PathBuf::from("jobs").join(&job.id),
            };
            jobs.push(ManifestJob {
                expanded_id,
                job_id: job.id.clone(),
                label: job.label.clone().unwrap_or_else(|| job.id.clone()),
                seed,
                status: JobStatus::Pending,
                attempt_count: 0,
                attempt_root,
                run: ManifestJobRun {
                    topology: job
                        .topology
                        .clone()
                        .or_else(|| config.defaults.topology.clone()),
                    parameters: parameters.clone(),
                    compare_parameters: job.compare_parameters.clone(),
                    slots: job.slots.or(config.defaults.slots),
                    timescale: job.timescale.or(config.defaults.timescale),
                    trace: job.trace.or(config.defaults.trace).unwrap_or(false),
                    trace_nodes: trace_nodes.clone(),
                    aggregate_events: job
                        .aggregate_events
                        .or(config.defaults.aggregate_events)
                        .unwrap_or(false),
                    conformance_events: job
                        .conformance_events
                        .or(config.defaults.conformance_events)
                        .unwrap_or(false),
                },
                attempts: Vec::new(),
            });
        }
    }

    Ok(jobs)
}

fn expanded_job_id(job_id: &str, seed: Option<u64>) -> String {
    match seed {
        Some(seed) => format!("{job_id}-seed-{seed}"),
        None => job_id.to_string(),
    }
}

fn resolve_suite_label(suite_path: &Path, label_override: Option<&str>) -> Result<String> {
    let raw = match label_override {
        Some(label) => label,
        None => suite_path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("suite"),
    };
    let label = runner::slugify(raw);
    if label.is_empty() {
        bail!("suite label resolved to an empty string");
    }
    Ok(label)
}

fn allocate_run_dir(output_root: &Path, label: &str) -> Result<PathBuf> {
    let timestamp = timestamp_string()?;
    let base_name = format!("{timestamp}-{label}");
    let candidate = output_root.join(&base_name);
    if !candidate.exists() {
        return Ok(candidate);
    }

    for attempt in 2..=9999 {
        let candidate = output_root.join(format!("{base_name}-{attempt}"));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }

    bail!(
        "could not allocate a unique suite run directory under {}",
        output_root.display()
    )
}

fn manifest_path(run_dir: &Path) -> PathBuf {
    run_dir.join(MANIFEST_FILE)
}

fn load_manifest(run_dir: &Path) -> Result<SuiteManifest> {
    let path = manifest_path(run_dir);
    let bytes = fs::read(&path)
        .with_context(|| format!("failed to read suite manifest {}", path.display()))?;
    let manifest = serde_json::from_slice(&bytes)
        .with_context(|| format!("failed to parse suite manifest {}", path.display()))?;
    Ok(manifest)
}

fn write_manifest(run_dir: &Path, manifest: &mut SuiteManifest) -> Result<()> {
    manifest.updated_at = now_rfc3339()?;
    let manifest_bytes = serde_json::to_vec_pretty(manifest)?;
    let path = manifest_path(run_dir);
    let temp_path = path.with_extension("json.tmp");
    fs::write(&temp_path, manifest_bytes)
        .with_context(|| format!("failed to write suite manifest {}", temp_path.display()))?;
    fs::rename(&temp_path, &path).with_context(|| {
        format!(
            "failed to move suite manifest into place ({} -> {})",
            temp_path.display(),
            path.display()
        )
    })?;
    Ok(())
}

async fn execute_manifest_jobs<F, Fut>(
    run_dir: &Path,
    manifest: &mut SuiteManifest,
    shutdown: CancellationToken,
    run_request_fn: &F,
) -> Result<()>
where
    F: Fn(RunRequest, CancellationToken) -> Fut,
    Fut: Future<Output = Result<RunOutcome>>,
{
    if manifest
        .jobs
        .iter()
        .all(|job| job.status == JobStatus::Completed)
    {
        manifest.status = SuiteStatus::Completed;
        write_manifest(run_dir, manifest)?;
        return Ok(());
    }

    manifest.status = SuiteStatus::Running;
    write_manifest(run_dir, manifest)?;

    for job_index in 0..manifest.jobs.len() {
        if manifest.jobs[job_index].status == JobStatus::Completed {
            continue;
        }

        if shutdown.is_cancelled() {
            manifest.status = SuiteStatus::Interrupted;
            write_manifest(run_dir, manifest)?;
            return Err(anyhow!("suite interrupted"));
        }

        let attempt_dir = next_attempt_dir(run_dir, &manifest.jobs[job_index])?;
        fs::create_dir_all(&attempt_dir).with_context(|| {
            format!(
                "failed to create attempt directory {}",
                attempt_dir.display()
            )
        })?;

        let started_at = now_rfc3339()?;
        {
            let job = &mut manifest.jobs[job_index];
            job.attempt_count += 1;
            job.status = JobStatus::Running;
            job.attempts.push(JobAttempt {
                index: job.attempt_count,
                status: JobStatus::Running,
                dir: relative_to(run_dir, &attempt_dir),
                started_at,
                finished_at: None,
                error: None,
                report_path: None,
                case_outputs: Vec::new(),
            });
        }
        write_manifest(run_dir, manifest)?;

        let request = match build_run_request_for_attempt(&manifest.jobs[job_index], &attempt_dir) {
            Ok(request) => request,
            Err(err) => {
                finish_job_attempt(
                    manifest,
                    job_index,
                    JobStatus::Failed,
                    Some(err.to_string()),
                    None,
                    Vec::new(),
                )?;
                manifest.status = SuiteStatus::Failed;
                write_manifest(run_dir, manifest)?;
                return Err(err);
            }
        };

        match run_request_fn(request, shutdown.clone()).await {
            Ok(outcome) => {
                if shutdown.is_cancelled() {
                    finish_job_attempt(
                        manifest,
                        job_index,
                        JobStatus::Interrupted,
                        Some("run cancelled".to_string()),
                        None,
                        Vec::new(),
                    )?;
                    manifest.status = SuiteStatus::Interrupted;
                    write_manifest(run_dir, manifest)?;
                    return Err(anyhow!("suite interrupted"));
                }

                let report_path = outcome
                    .comparison_output
                    .as_ref()
                    .map(|path| relative_to(run_dir, path));
                let case_outputs = outcome
                    .cases
                    .iter()
                    .map(|case| JobAttemptCaseOutput {
                        label: case.label.clone(),
                        output_path: case
                            .output_path
                            .as_ref()
                            .map(|path| relative_to(run_dir, path)),
                    })
                    .collect();
                finish_job_attempt(
                    manifest,
                    job_index,
                    JobStatus::Completed,
                    None,
                    report_path,
                    case_outputs,
                )?;
                write_manifest(run_dir, manifest)?;
            }
            Err(err) => {
                let interrupted = shutdown.is_cancelled() || runner::is_run_interrupted(&err);
                let status = if interrupted {
                    JobStatus::Interrupted
                } else {
                    JobStatus::Failed
                };
                finish_job_attempt(
                    manifest,
                    job_index,
                    status.clone(),
                    Some(err.to_string()),
                    None,
                    Vec::new(),
                )?;
                manifest.status = if interrupted {
                    SuiteStatus::Interrupted
                } else {
                    SuiteStatus::Failed
                };
                write_manifest(run_dir, manifest)?;
                return Err(err);
            }
        }
    }

    manifest.status = SuiteStatus::Completed;
    write_manifest(run_dir, manifest)?;
    Ok(())
}

fn next_attempt_dir(run_dir: &Path, job: &ManifestJob) -> Result<PathBuf> {
    for index in (job.attempt_count + 1)..=(job.attempt_count + 1024) {
        let candidate = run_dir
            .join(&job.attempt_root)
            .join(format!("attempt-{index:03}"));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    bail!(
        "could not allocate a new attempt directory for job '{}'",
        job.expanded_id
    )
}

fn build_run_request_for_attempt(job: &ManifestJob, attempt_dir: &Path) -> Result<RunRequest> {
    let mut trailing_parameters = Vec::new();
    if let Some(seed) = job.seed {
        let seed_override_path = attempt_dir.join("seed.override.yaml");
        fs::write(&seed_override_path, format!("seed: {seed}\n")).with_context(|| {
            format!(
                "failed to write generated seed override {}",
                seed_override_path.display()
            )
        })?;
        trailing_parameters.push(seed_override_path);
    }

    Ok(RunRequest {
        topology: job.run.topology.clone(),
        output: Some(attempt_dir.join("events.jsonl")),
        parameters: job.run.parameters.clone(),
        compare_parameters: job.run.compare_parameters.clone(),
        comparison_output: (!job.run.compare_parameters.is_empty())
            .then_some(attempt_dir.join("report.txt")),
        timescale: job.run.timescale,
        trace_nodes: job.run.trace_nodes.clone(),
        slots: job.run.slots,
        conformance_events: job.run.conformance_events,
        aggregate_events: job.run.aggregate_events,
        no_trace: !job.run.trace,
        trailing_parameters,
    })
}

fn finish_job_attempt(
    manifest: &mut SuiteManifest,
    job_index: usize,
    status: JobStatus,
    error: Option<String>,
    report_path: Option<PathBuf>,
    case_outputs: Vec<JobAttemptCaseOutput>,
) -> Result<()> {
    let finished_at = now_rfc3339()?;
    let job = &mut manifest.jobs[job_index];
    job.status = status.clone();
    let attempt = job
        .attempts
        .last_mut()
        .ok_or_else(|| anyhow!("job '{}' has no attempt to finish", job.expanded_id))?;
    attempt.status = status;
    attempt.finished_at = Some(finished_at);
    attempt.error = error;
    attempt.report_path = report_path;
    attempt.case_outputs = case_outputs;
    Ok(())
}

fn mark_stale_running_jobs(manifest: &mut SuiteManifest) -> Result<bool> {
    let mut changed = false;
    for job in &mut manifest.jobs {
        if job.status == JobStatus::Running {
            job.status = JobStatus::Interrupted;
            changed = true;
        }
        if let Some(attempt) = job.attempts.last_mut()
            && attempt.status == JobStatus::Running
        {
            attempt.status = JobStatus::Interrupted;
            if attempt.finished_at.is_none() {
                attempt.finished_at = Some(now_rfc3339()?);
            }
            if attempt.error.is_none() {
                attempt.error = Some("suite resumed after prior process termination".to_string());
            }
            changed = true;
        }
    }
    if changed && manifest.status == SuiteStatus::Running {
        manifest.status = SuiteStatus::Interrupted;
    }
    Ok(changed)
}

fn relative_to(base: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(base)
        .map(PathBuf::from)
        .unwrap_or_else(|_| path.to_path_buf())
}

fn timestamp_string() -> Result<String> {
    let format = time::format_description::parse("[year][month][day]-[hour][minute][second]")?;
    Ok(OffsetDateTime::now_utc().format(&format)?)
}

fn now_rfc3339() -> Result<String> {
    Ok(OffsetDateTime::now_utc().format(&Rfc3339)?)
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

    use tokio::{task::JoinHandle, time::sleep};

    use super::*;
    use crate::runner::RunCaseOutcome;

    fn unique_temp_dir(label: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "experiment-suite-{label}-{}-{suffix}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("sim-rs root")
            .to_path_buf()
    }

    fn write_suite(dir: &Path, name: &str, yaml: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, yaml).expect("write suite");
        path
    }

    fn write_experiment(
        dir: &Path,
        name: &str,
        pricing_config: &Path,
        actors_config: &Path,
    ) -> PathBuf {
        let path = dir.join(name);
        fs::write(
            &path,
            format!(
                r#"leios-variant: linear
tx-generator: actors
enforce-tier-delay: true
seed: 42
leios-header-diffusion-time-ms: 2000.0
linear-vote-stage-length-slots: 3
linear-diffuse-stage-length-slots: 4
vote-threshold: 180
eb-referenced-txs-max-size-bytes: 16384000
pricing:
  config-path: "{}"
actors:
  config-path: "{}"
"#,
                pricing_config.display(),
                actors_config.display(),
            ),
        )
        .expect("write experiment");
        path
    }

    fn read_manifest_for_test(run_dir: &Path) -> SuiteManifest {
        load_manifest(run_dir).expect("load manifest")
    }

    fn build_success_outcome(request: &RunRequest) -> Result<RunOutcome> {
        if let Some(output) = &request.output {
            if let Some(parent) = output.parent() {
                fs::create_dir_all(parent)?;
            }
            if !request.no_trace {
                fs::write(output, "trace\n")?;
            }
            let metrics_dir = output.parent().unwrap_or_else(|| Path::new("."));
            fs::write(metrics_dir.join("metrics_comparison.txt"), "metrics\n")?;
        }

        if request.compare_parameters.is_empty() {
            return Ok(RunOutcome {
                cases: vec![RunCaseOutcome {
                    label: "run".to_string(),
                    parameters: request.parameters.clone(),
                    output_path: request.output.clone(),
                    summary: crate::events::RunSummary {
                        submissions: 0,
                        unique_generated: 0,
                        unique_generated_bytes: 0,
                        rejected: 0,
                        included: 0,
                        included_bytes: 0,
                        optimal_supply_capacity_bytes: 0,
                        optimal_included_bytes: 0,
                        included_vs_generated_bytes_ratio: 0.0,
                        included_vs_optimal_bytes_ratio: 0.0,
                        inclusion_rate: 0.0,
                        unique_inclusion_rate: 0.0,
                        tier_delay_unit: sim_core::config::TierDelayUnit::Blocks,
                        latency_mean_slots: 0.0,
                        latency_p95_slots: 0.0,
                        latency_p99_slots: 0.0,
                        fees_total: 0,
                        fee_per_byte: 0.0,
                        fee_per_tx: 0.0,
                        retained_value_total: 0,
                        retained_value_ratio_generated: 0.0,
                        retained_value_ratio_settled: 0.0,
                        net_utility_total: 0,
                        net_utility_per_generated_tx: 0.0,
                        rb_generated: 0,
                        eb_generated: 0,
                        max_tier_count: 0,
                    },
                }],
                comparison_output: None,
            });
        }

        let mut cases = Vec::new();
        if let Some(output) = &request.output {
            let baseline_output = output
                .parent()
                .expect("attempt dir")
                .join("baseline")
                .join("events.jsonl");
            fs::create_dir_all(baseline_output.parent().expect("baseline dir"))?;
            cases.push(RunCaseOutcome {
                label: "baseline".to_string(),
                parameters: request.parameters.clone(),
                output_path: Some(baseline_output),
                summary: build_success_outcome(&RunRequest::default())?.cases[0]
                    .summary
                    .clone(),
            });
            for compare in &request.compare_parameters {
                let label = compare
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .unwrap_or("case")
                    .to_string();
                let case_output = output
                    .parent()
                    .expect("attempt dir")
                    .join(&label)
                    .join("events.jsonl");
                fs::create_dir_all(case_output.parent().expect("case dir"))?;
                cases.push(RunCaseOutcome {
                    label,
                    parameters: Vec::new(),
                    output_path: Some(case_output),
                    summary: build_success_outcome(&RunRequest::default())?.cases[0]
                        .summary
                        .clone(),
                });
            }
        }

        if let Some(report_path) = &request.comparison_output {
            if let Some(parent) = report_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(report_path, "report\n")?;
        }

        Ok(RunOutcome {
            cases,
            comparison_output: request.comparison_output.clone(),
        })
    }

    #[test]
    fn validates_duplicate_ids_and_expands_defaults() -> Result<()> {
        let config: SuiteConfig = serde_yaml::from_str(
            r#"
defaults:
  parameters:
    - base-a.yaml
  trace: true
  trace-nodes: [1]
jobs:
  - id: alpha
    parameters:
      - job-a.yaml
    seeds: [42, 43]
  - id: beta
    trace: false
    trace-nodes: [3]
"#,
        )?;
        validate_suite_config(&config)?;
        let jobs = expand_jobs(&config)?;
        assert_eq!(jobs.len(), 3);
        assert_eq!(
            jobs[0].run.parameters,
            vec![PathBuf::from("base-a.yaml"), PathBuf::from("job-a.yaml")]
        );
        assert_eq!(jobs[0].seed, Some(42));
        assert_eq!(jobs[0].run.trace_nodes, vec![1]);
        assert!(!jobs[2].run.trace);
        assert_eq!(jobs[2].run.trace_nodes, vec![1, 3]);

        let dupes: SuiteConfig = serde_yaml::from_str(
            r#"
jobs:
  - id: alpha
  - id: alpha
"#,
        )?;
        assert!(validate_suite_config(&dupes).is_err());
        Ok(())
    }

    #[tokio::test]
    async fn runs_single_job_suite_with_real_runner() -> Result<()> {
        let temp_dir = unique_temp_dir("real-single");
        let output_root = temp_dir.join("output");
        let sim_root = repo_root();
        let baseline_experiment = write_experiment(
            &temp_dir,
            "baseline.yaml",
            &sim_root.join("parameters/phase-2-sweep/pricing/baseline_quick.toml"),
            &sim_root.join("parameters/phase-2-sweep/demand/paper_like_moderate.toml"),
        );
        let suite_path = write_suite(
            &temp_dir,
            "single.yaml",
            &format!(
                r#"
jobs:
  - id: single
    parameters:
      - {}
      - {}
    slots: 1
"#,
                sim_root.join("parameters/linear.yaml").display(),
                baseline_experiment.display(),
            ),
        );

        let run_dir = run_suite(
            &suite_path,
            None,
            Some(output_root.as_path()),
            CancellationToken::new(),
        )
        .await?;

        let manifest = read_manifest_for_test(&run_dir);
        assert_eq!(manifest.status, SuiteStatus::Completed);
        assert_eq!(manifest.jobs.len(), 1);
        assert_eq!(manifest.jobs[0].status, JobStatus::Completed);
        assert!(run_dir.join("suite.yaml").exists());
        assert!(
            run_dir
                .join(&manifest.jobs[0].attempts[0].dir)
                .join("metrics_comparison.txt")
                .exists()
        );
        Ok(())
    }

    #[tokio::test]
    async fn runs_comparison_suite_with_real_runner() -> Result<()> {
        let temp_dir = unique_temp_dir("real-compare");
        let output_root = temp_dir.join("output");
        let sim_root = repo_root();
        let baseline_experiment = write_experiment(
            &temp_dir,
            "baseline.yaml",
            &sim_root.join("parameters/phase-2-sweep/pricing/baseline_quick.toml"),
            &sim_root.join("parameters/phase-2-sweep/demand/paper_like_moderate.toml"),
        );
        let compare_one = write_experiment(
            &temp_dir,
            "compare-one.yaml",
            &sim_root.join("parameters/phase-2-sweep/pricing/tiered_quick.toml"),
            &sim_root.join("parameters/phase-2-sweep/demand/paper_like_moderate.toml"),
        );
        let compare_two = write_experiment(
            &temp_dir,
            "compare-two.yaml",
            &sim_root.join("parameters/phase-2-sweep/pricing/eip1559_quick.toml"),
            &sim_root.join("parameters/phase-2-sweep/demand/paper_like_moderate.toml"),
        );
        let suite_path = write_suite(
            &temp_dir,
            "compare.yaml",
            &format!(
                r#"
jobs:
  - id: compare
    parameters:
      - {}
      - {}
    compare-parameters:
      - {}
      - {}
    slots: 1
"#,
                sim_root.join("parameters/linear.yaml").display(),
                baseline_experiment.display(),
                compare_one.display(),
                compare_two.display(),
            ),
        );

        let run_dir = run_suite(
            &suite_path,
            None,
            Some(output_root.as_path()),
            CancellationToken::new(),
        )
        .await?;
        let manifest = read_manifest_for_test(&run_dir);
        let attempt = &manifest.jobs[0].attempts[0];
        assert_eq!(manifest.jobs[0].status, JobStatus::Completed);
        assert!(attempt.report_path.is_some());
        assert_eq!(attempt.case_outputs.len(), 3);
        Ok(())
    }

    #[tokio::test]
    async fn interrupt_and_resume_reruns_only_unfinished_jobs() -> Result<()> {
        let temp_dir = unique_temp_dir("interrupt");
        let output_root = temp_dir.join("output");
        let suite_path = write_suite(
            &temp_dir,
            "interrupt.yaml",
            r#"
jobs:
  - id: first
  - id: second
"#,
        );
        let cancel_token = CancellationToken::new();
        let runner_calls = Arc::new(AtomicUsize::new(0));
        let wait_calls = runner_calls.clone();
        let run_handle: JoinHandle<Result<PathBuf>> = tokio::spawn({
            let suite_path = suite_path.clone();
            let output_root = output_root.clone();
            let cancel_token = cancel_token.clone();
            async move {
                run_suite_with_runner(
                    &suite_path,
                    None,
                    Some(output_root.as_path()),
                    cancel_token,
                    &move |request, token| {
                        let wait_calls = wait_calls.clone();
                        async move {
                            wait_calls.fetch_add(1, Ordering::SeqCst);
                            if let Some(output) = &request.output {
                                fs::create_dir_all(output.parent().expect("attempt dir"))?;
                                fs::write(
                                    output.parent().expect("attempt dir").join("partial.txt"),
                                    "partial\n",
                                )?;
                            }
                            token.cancelled().await;
                            Err(anyhow!("cancelled"))
                        }
                    },
                )
                .await
            }
        });

        let run_dir = loop {
            if let Some(entry) = fs::read_dir(&output_root)
                .ok()
                .and_then(|mut entries| entries.next())
                .transpose()?
            {
                break entry.path();
            }
            sleep(Duration::from_millis(10)).await;
        };

        loop {
            let manifest = read_manifest_for_test(&run_dir);
            if manifest
                .jobs
                .iter()
                .any(|job| job.status == JobStatus::Running)
            {
                break;
            }
            sleep(Duration::from_millis(10)).await;
        }

        cancel_token.cancel();
        assert!(run_handle.await.expect("join").is_err());

        let interrupted_manifest = read_manifest_for_test(&run_dir);
        assert_eq!(interrupted_manifest.status, SuiteStatus::Interrupted);
        assert_eq!(interrupted_manifest.jobs[0].status, JobStatus::Interrupted);
        assert_eq!(interrupted_manifest.jobs[1].status, JobStatus::Pending);
        assert_eq!(runner_calls.load(Ordering::SeqCst), 1);

        let resume_calls = Arc::new(AtomicUsize::new(0));
        let resume_calls_for_runner = resume_calls.clone();
        resume_suite_with_runner(
            &run_dir,
            CancellationToken::new(),
            &move |request, _token| {
                let resume_calls_for_runner = resume_calls_for_runner.clone();
                async move {
                    resume_calls_for_runner.fetch_add(1, Ordering::SeqCst);
                    build_success_outcome(&request)
                }
            },
        )
        .await?;

        let resumed_manifest = read_manifest_for_test(&run_dir);
        assert_eq!(resumed_manifest.status, SuiteStatus::Completed);
        assert_eq!(resumed_manifest.jobs[0].attempt_count, 2);
        assert_eq!(resumed_manifest.jobs[1].attempt_count, 1);
        assert_eq!(resume_calls.load(Ordering::SeqCst), 2);
        Ok(())
    }

    #[tokio::test]
    async fn failure_stops_suite_and_resume_retries_failed_job() -> Result<()> {
        let temp_dir = unique_temp_dir("failure");
        let output_root = temp_dir.join("output");
        let suite_path = write_suite(
            &temp_dir,
            "failure.yaml",
            r#"
jobs:
  - id: first
  - id: second
"#,
        );
        let failed_once = Arc::new(AtomicUsize::new(0));
        let run_result = run_suite_with_runner(
            &suite_path,
            None,
            Some(output_root.as_path()),
            CancellationToken::new(),
            &move |_request, _token| {
                let failed_once = failed_once.clone();
                async move {
                    if failed_once.fetch_add(1, Ordering::SeqCst) == 0 {
                        Err(anyhow!("missing parameter path"))
                    } else {
                        Ok(RunOutcome::default())
                    }
                }
            },
        )
        .await;
        assert!(run_result.is_err());

        let run_dir = fs::read_dir(&output_root)?
            .next()
            .transpose()?
            .expect("suite run")
            .path();
        let failed_manifest = read_manifest_for_test(&run_dir);
        assert_eq!(failed_manifest.status, SuiteStatus::Failed);
        assert_eq!(failed_manifest.jobs[0].status, JobStatus::Failed);
        assert_eq!(failed_manifest.jobs[1].status, JobStatus::Pending);

        let resume_calls = Arc::new(AtomicUsize::new(0));
        let resume_calls_for_runner = resume_calls.clone();
        resume_suite_with_runner(
            &run_dir,
            CancellationToken::new(),
            &move |request, _token| {
                let resume_calls = resume_calls_for_runner.clone();
                async move {
                    resume_calls.fetch_add(1, Ordering::SeqCst);
                    build_success_outcome(&request)
                }
            },
        )
        .await?;

        let resumed_manifest = read_manifest_for_test(&run_dir);
        assert_eq!(resumed_manifest.status, SuiteStatus::Completed);
        assert_eq!(resumed_manifest.jobs[0].attempt_count, 2);
        assert_eq!(resumed_manifest.jobs[1].attempt_count, 1);
        assert_eq!(resume_calls.load(Ordering::SeqCst), 2);
        Ok(())
    }
}
