use std::{
    fs,
    path::PathBuf,
    process,
    sync::{
        Arc, Once,
        atomic::{AtomicBool, Ordering},
    },
};

use anyhow::{Result, anyhow, bail};
use figment::{
    Figment,
    providers::{Format as _, Yaml},
};
use sim_core::{
    clock::ClockCoordinator,
    config::{NodeId, RawParameters, RawTopology, SimConfiguration, TierDelayUnit, Topology},
    events::EventTracker,
    sim::Simulation,
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{info, level_filters::LevelFilter, warn};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt as _, util::SubscriberInitExt};

use crate::events::{EventMonitor, RunSummary};

pub const DEFAULT_TOPOLOGY_PATHS: &[&str] = &[
    // Standalone sim-rs path (run from sim-rs directory)
    "parameters/topology.default.yaml",
    // Standalone sim-rs path (run from sim-rs/sim-cli directory)
    "../parameters/topology.default.yaml",
    // Docker/production path
    "/usr/local/share/leios/topology.default.yaml",
    // Legacy monorepo development paths
    "../../data/simulation/topo-default-100.yaml",
    "../data/simulation/topo-default-100.yaml",
];
const EMBEDDED_DEFAULT_TOPOLOGY: &str = include_str!("../../parameters/topology.default.yaml");

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RunRequest {
    pub topology: Option<PathBuf>,
    pub output: Option<PathBuf>,
    pub parameters: Vec<PathBuf>,
    pub compare_parameters: Vec<PathBuf>,
    pub comparison_output: Option<PathBuf>,
    pub timescale: Option<f64>,
    pub trace_nodes: Vec<usize>,
    pub slots: Option<u64>,
    pub conformance_events: bool,
    pub aggregate_events: bool,
    pub no_trace: bool,
    pub trailing_parameters: Vec<PathBuf>,
}

#[derive(Clone, Debug)]
pub struct RunCaseOutcome {
    pub label: String,
    pub parameters: Vec<PathBuf>,
    pub output_path: Option<PathBuf>,
    pub summary: RunSummary,
}

#[derive(Clone, Debug, Default)]
pub struct RunOutcome {
    pub cases: Vec<RunCaseOutcome>,
    pub comparison_output: Option<PathBuf>,
}

#[derive(Clone)]
struct ComparisonCase {
    label: String,
    extra_parameters: Vec<PathBuf>,
}

#[derive(Clone)]
struct ComparisonResult {
    label: String,
    parameters: Vec<PathBuf>,
    output_path: Option<PathBuf>,
    summary: RunSummary,
}

#[derive(Debug)]
struct RunInterrupted;

impl std::fmt::Display for RunInterrupted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("run cancelled")
    }
}

impl std::error::Error for RunInterrupted {}

pub fn init_tracing() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let fmt_layer = tracing_subscriber::fmt::layer().compact().without_time();
        let filter = EnvFilter::builder()
            .with_default_directive(LevelFilter::INFO.into())
            .from_env_lossy();
        tracing_subscriber::registry()
            .with(fmt_layer)
            .with(filter)
            .init();
    });
}

pub fn install_ctrlc_handler(token: CancellationToken) -> Result<()> {
    let interrupted = Arc::new(AtomicBool::new(false));
    let interrupted_flag = interrupted.clone();
    ctrlc::set_handler(move || {
        token.cancel();
        if interrupted_flag.swap(true, Ordering::SeqCst) {
            warn!("force quitting");
            process::exit(0);
        }
    })?;
    Ok(())
}

pub fn is_run_interrupted(err: &anyhow::Error) -> bool {
    err.downcast_ref::<RunInterrupted>().is_some()
}

pub fn slugify(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut prev_dash = false;
    for ch in input.chars() {
        let keep = ch.is_ascii_alphanumeric() || ch == '_' || ch == '-';
        if keep {
            out.push(ch.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

fn interrupted_error() -> anyhow::Error {
    RunInterrupted.into()
}

fn validate_run_request(request: &RunRequest) -> Result<()> {
    if let Some(timescale) = request.timescale
        && (!timescale.is_finite() || timescale <= 0.0)
    {
        bail!("timescale must be a positive finite number");
    }
    Ok(())
}

fn get_default_topology() -> Result<String> {
    for path in DEFAULT_TOPOLOGY_PATHS {
        if let Ok(content) = fs::read_to_string(path) {
            return Ok(content);
        }
    }
    Ok(EMBEDDED_DEFAULT_TOPOLOGY.to_string())
}

pub fn read_config(request: &RunRequest, extra_parameters: &[PathBuf]) -> Result<SimConfiguration> {
    let topology_str = match &request.topology {
        Some(path) => fs::read_to_string(path)?,
        None => get_default_topology()?,
    };
    let topology: Topology = {
        let raw_topology: RawTopology = serde_yaml::from_str(&topology_str)?;
        raw_topology.into()
    };
    topology.validate()?;

    let mut raw_params = Figment::new().merge(Yaml::string(include_str!(
        "../../parameters/config.default.yaml"
    )));

    for params_file in request
        .parameters
        .iter()
        .chain(extra_parameters.iter())
        .chain(request.trailing_parameters.iter())
    {
        raw_params = raw_params.merge(Yaml::file_exact(params_file));
    }

    let params: RawParameters = raw_params.extract()?;
    let mut config = SimConfiguration::build(params, topology)?;
    if let Some(slots) = request.slots {
        config.slots = Some(slots);
    }
    if request.conformance_events {
        config.emit_conformance_events = true;
    }
    if request.aggregate_events {
        config.aggregate_events = true;
    }
    for id in &request.trace_nodes {
        config.trace_nodes.insert(NodeId::new(*id));
    }
    Ok(config)
}

fn comparison_cases(request: &RunRequest) -> Vec<ComparisonCase> {
    let mut cases = vec![ComparisonCase {
        label: "baseline".to_string(),
        extra_parameters: Vec::new(),
    }];
    for (index, path) in request.compare_parameters.iter().enumerate() {
        let label = case_label(path, index);
        cases.push(ComparisonCase {
            label,
            extra_parameters: vec![path.clone()],
        });
    }
    cases
}

fn case_label(path: &PathBuf, index: usize) -> String {
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("case");
    let slug = slugify(stem);
    if slug.is_empty() {
        format!("case-{}", index + 1)
    } else {
        slug
    }
}

fn case_output_path(base_output: &Option<PathBuf>, label: &str) -> Option<PathBuf> {
    let base = base_output.as_ref()?;
    let safe_label = slugify(label);
    let parent = base
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let case_dir = parent.join(safe_label);
    let stem = base
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("sim")
        .to_string();
    match base.extension().and_then(|e| e.to_str()) {
        Some(ext) => Some(case_dir.join(format!("{stem}.{ext}"))),
        None => Some(case_dir.join(format!("{stem}.jsonl"))),
    }
}

fn tier_delay_unit_label(unit: TierDelayUnit) -> &'static str {
    match unit {
        TierDelayUnit::Slots => "slots",
        TierDelayUnit::Blocks => "blocks",
    }
}

fn comparison_output_path(request: &RunRequest) -> PathBuf {
    if let Some(path) = &request.comparison_output {
        return path.clone();
    }
    if let Some(output) = &request.output {
        let parent = output
            .parent()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        let stem = output.file_stem().and_then(|s| s.to_str()).unwrap_or("sim");
        return parent.join(format!("{stem}-comparison.txt"));
    }
    PathBuf::from("sim-cli-comparison.txt")
}

fn format_comparison_report(results: &[ComparisonResult]) -> String {
    use std::fmt::Write as _;

    let mut out = String::new();
    let metric_width = 28usize;
    let col_width = 18usize;

    let mut write_row = |metric: &str, values: Vec<String>| {
        let _ = write!(
            out,
            "{:<metric_width$} |",
            metric,
            metric_width = metric_width
        );
        for value in values {
            let _ = write!(out, " {:>col_width$} |", value, col_width = col_width);
        }
        let _ = writeln!(out);
    };

    write_row(
        "Metric",
        results.iter().map(|result| result.label.clone()).collect(),
    );
    write_row(
        "----------------------------",
        results
            .iter()
            .map(|_| "-".repeat(col_width))
            .collect::<Vec<_>>(),
    );
    write_row(
        "Submissions",
        results
            .iter()
            .map(|result| result.summary.submissions.to_string())
            .collect(),
    );
    write_row(
        "Unique txs generated",
        results
            .iter()
            .map(|result| result.summary.unique_generated.to_string())
            .collect(),
    );
    write_row(
        "Generated bytes",
        results
            .iter()
            .map(|result| result.summary.unique_generated_bytes.to_string())
            .collect(),
    );
    write_row(
        "Rejected",
        results
            .iter()
            .map(|result| result.summary.rejected.to_string())
            .collect(),
    );
    write_row(
        "Included",
        results
            .iter()
            .map(|result| result.summary.included.to_string())
            .collect(),
    );
    write_row(
        "Included bytes",
        results
            .iter()
            .map(|result| result.summary.included_bytes.to_string())
            .collect(),
    );
    write_row(
        "Optimal supply bytes",
        results
            .iter()
            .map(|result| result.summary.optimal_supply_capacity_bytes.to_string())
            .collect(),
    );
    write_row(
        "Optimal incl. bytes",
        results
            .iter()
            .map(|result| result.summary.optimal_included_bytes.to_string())
            .collect(),
    );
    write_row(
        "Incl./generated bytes",
        results
            .iter()
            .map(|result| {
                format!(
                    "{:.2}%",
                    result.summary.included_vs_generated_bytes_ratio * 100.0
                )
            })
            .collect(),
    );
    write_row(
        "Incl./optimal bytes",
        results
            .iter()
            .map(|result| {
                format!(
                    "{:.2}%",
                    result.summary.included_vs_optimal_bytes_ratio * 100.0
                )
            })
            .collect(),
    );
    write_row(
        "Inclusion rate",
        results
            .iter()
            .map(|result| format!("{:.2}%", result.summary.inclusion_rate * 100.0))
            .collect(),
    );
    write_row(
        "Unique tx incl. rate",
        results
            .iter()
            .map(|result| format!("{:.2}%", result.summary.unique_inclusion_rate * 100.0))
            .collect(),
    );
    write_row(
        "Synthetic delay unit",
        results
            .iter()
            .map(|result| tier_delay_unit_label(result.summary.tier_delay_unit).to_string())
            .collect(),
    );
    write_row(
        "Latency mean (slots)",
        results
            .iter()
            .map(|result| format!("{:.2}", result.summary.latency_mean_slots))
            .collect(),
    );
    write_row(
        "Latency p95 (slots)",
        results
            .iter()
            .map(|result| format!("{:.2}", result.summary.latency_p95_slots))
            .collect(),
    );
    write_row(
        "Latency p99 (slots)",
        results
            .iter()
            .map(|result| format!("{:.2}", result.summary.latency_p99_slots))
            .collect(),
    );
    write_row(
        "Fees total",
        results
            .iter()
            .map(|result| result.summary.fees_total.to_string())
            .collect(),
    );
    write_row(
        "Fee per byte",
        results
            .iter()
            .map(|result| format!("{:.4}", result.summary.fee_per_byte))
            .collect(),
    );
    write_row(
        "Fee per tx",
        results
            .iter()
            .map(|result| format!("{:.4}", result.summary.fee_per_tx))
            .collect(),
    );
    write_row(
        "Retained value total",
        results
            .iter()
            .map(|result| result.summary.retained_value_total.to_string())
            .collect(),
    );
    write_row(
        "Retained/gen value",
        results
            .iter()
            .map(|result| {
                format!(
                    "{:.2}%",
                    result.summary.retained_value_ratio_generated * 100.0
                )
            })
            .collect(),
    );
    write_row(
        "Retained/included init",
        results
            .iter()
            .map(|result| {
                format!(
                    "{:.2}%",
                    result.summary.retained_value_ratio_settled * 100.0
                )
            })
            .collect(),
    );
    write_row(
        "Net utility total",
        results
            .iter()
            .map(|result| result.summary.net_utility_total.to_string())
            .collect(),
    );
    write_row(
        "Net util/gen tx",
        results
            .iter()
            .map(|result| format!("{:.4}", result.summary.net_utility_per_generated_tx))
            .collect(),
    );
    write_row(
        "RB generated",
        results
            .iter()
            .map(|result| result.summary.rb_generated.to_string())
            .collect(),
    );
    write_row(
        "EB generated",
        results
            .iter()
            .map(|result| result.summary.eb_generated.to_string())
            .collect(),
    );
    write_row(
        "Max tier count",
        results
            .iter()
            .map(|result| result.summary.max_tier_count.to_string())
            .collect(),
    );

    let _ = writeln!(out);
    let _ = writeln!(out, "Run details:");
    for result in results {
        let parameters = result
            .parameters
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let output_path = result
            .output_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "<none>".to_string());
        let _ = writeln!(out, "- Case: {}", result.label);
        let _ = writeln!(out, "  Parameters: {}", parameters);
        let _ = writeln!(out, "  Output trace: {}", output_path);
    }

    out
}

async fn run_single_simulation(
    config: SimConfiguration,
    output_path: Option<PathBuf>,
    write_trace: bool,
    shutdown: CancellationToken,
) -> Result<RunSummary> {
    let (events_sink, events_source) = mpsc::unbounded_channel();
    let monitor = tokio::spawn(
        EventMonitor::new(
            &config,
            events_source,
            output_path,
            write_trace,
            shutdown.clone(),
        )
        .run(),
    );

    let clock_coordinator = ClockCoordinator::new(config.timestamp_resolution);
    let clock = clock_coordinator.clock();
    let tracker = EventTracker::new(events_sink, clock.clone(), &config.nodes);
    let mut simulation = Simulation::new(config, tracker, clock_coordinator).await?;

    let run_result = simulation.run(shutdown).await;
    simulation.shutdown()?;
    let summary = monitor.await??;
    run_result?;
    Ok(summary)
}

pub async fn execute_run_request(
    request: &RunRequest,
    token: CancellationToken,
) -> Result<RunOutcome> {
    validate_run_request(request)?;

    if request.compare_parameters.is_empty() {
        let config = read_config(request, &[])?;
        let output_path = request.output.clone();
        let summary = run_single_simulation(
            config,
            output_path.clone(),
            !request.no_trace,
            token.clone(),
        )
        .await?;
        if token.is_cancelled() {
            return Err(interrupted_error());
        }
        let mut parameters = request.parameters.clone();
        parameters.extend(request.trailing_parameters.clone());
        return Ok(RunOutcome {
            cases: vec![RunCaseOutcome {
                label: "run".to_string(),
                parameters,
                output_path,
                summary,
            }],
            comparison_output: None,
        });
    }

    let mut results = Vec::new();
    for case in comparison_cases(request) {
        if token.is_cancelled() {
            return Err(interrupted_error());
        }
        let config = read_config(request, &case.extra_parameters)?;
        let output_path = case_output_path(&request.output, &case.label);
        let summary = run_single_simulation(
            config,
            output_path.clone(),
            !request.no_trace,
            token.clone(),
        )
        .await?;
        if token.is_cancelled() {
            return Err(interrupted_error());
        }
        let mut merged_parameters = request.parameters.clone();
        merged_parameters.extend(case.extra_parameters.iter().cloned());
        merged_parameters.extend(request.trailing_parameters.iter().cloned());
        results.push(ComparisonResult {
            label: case.label,
            parameters: merged_parameters,
            output_path,
            summary,
        });
    }

    if results.is_empty() {
        return Err(anyhow!("no runs were executed"));
    }

    let report_path = comparison_output_path(request);
    if let Some(parent) = report_path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    let report = format_comparison_report(&results);
    fs::write(&report_path, report)?;
    info!("wrote comparison report to {}", report_path.display());

    Ok(RunOutcome {
        comparison_output: Some(report_path),
        cases: results
            .into_iter()
            .map(|result| RunCaseOutcome {
                label: result.label,
                parameters: result.parameters,
                output_path: result.output_path,
                summary: result.summary,
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use std::{fs, path::PathBuf};

    use super::{RunRequest, read_config};

    #[test]
    fn should_parse_topologies() -> Result<()> {
        let topology_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../test_data");
        for topology in fs::read_dir(topology_dir)? {
            let request = RunRequest {
                topology: Some(topology?.path()),
                ..RunRequest::default()
            };
            read_config(&request, &[])?;
        }
        Ok(())
    }

    #[test]
    fn trailing_parameters_override_compare_case_inputs() -> Result<()> {
        let temp_dir =
            std::env::temp_dir().join(format!("sim-cli-runner-test-{}", std::process::id()));
        fs::create_dir_all(&temp_dir)?;
        let base = temp_dir.join("base.yaml");
        let compare = temp_dir.join("compare.yaml");
        let trailing = temp_dir.join("trailing.yaml");
        fs::write(&base, "seed: 1\n")?;
        fs::write(&compare, "seed: 2\n")?;
        fs::write(&trailing, "seed: 3\n")?;

        let request = RunRequest {
            topology: Some(PathBuf::from("../test_data/small.yaml")),
            parameters: vec![base],
            compare_parameters: vec![compare],
            trailing_parameters: vec![trailing],
            ..RunRequest::default()
        };

        let config = read_config(&request, &request.compare_parameters)?;
        assert_eq!(config.seed, 3);
        Ok(())
    }
}
