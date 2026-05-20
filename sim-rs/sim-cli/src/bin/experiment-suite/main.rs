//! Phase-2 experiment-suite runner. M3.
//!
//! `experiment-suite run <suite.yaml>` — load the suite, expand its
//! (job × seed) cartesian product, run each unfinished job, and
//! write per-job artefacts plus a per-suite metrics_comparison.
//! Resumable: manifest at `<output_dir>/manifest.json` records
//! status; re-running skips completed jobs.

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use sim_cli::{
    runner::{Manifest, apply_run_id, run_suite_with_run_id, verify_suite_with_run_id},
    suite::Suite,
};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt as _, util::SubscriberInitExt};

#[derive(Parser)]
#[command(version = concat!(env!("CARGO_PKG_VERSION"), "-", env!("VERGEN_GIT_SHA")))]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run all (or remaining) jobs of a suite. Resumable.
    Run {
        suite: PathBuf,
        /// Suffix appended to the suite's output_dir as
        /// `<output_dir>-<RUN_ID>`. The wrapper script
        /// `scripts/run-parallel-suites.sh` generates one timestamp
        /// at start and passes it to every spawned suite so they
        /// share a batch identifier. Re-running with the same ID
        /// resumes; a new ID starts a fresh dir.
        #[arg(long)]
        run_id: Option<String>,
        /// Max concurrent (job, seed) pairs. Default:
        /// `min(available_parallelism(), 8)`. Each parallel job
        /// owns its own simulator state, so peak RSS scales
        /// linearly in N — raise carefully if your topology is large
        /// or RAM is tight.
        #[arg(long, short = 'P')]
        parallelism: Option<usize>,
    },
    /// Print the manifest's per-job status without running.
    Status {
        suite: PathBuf,
        #[arg(long)]
        run_id: Option<String>,
    },
    /// Re-run every Completed (job, seed) and assert each
    /// freshly-computed pricing-event-stream SHA256 matches the
    /// persisted value. Plan §M3 verification line 321: the
    /// inline determinism check inside the runner.
    Verify {
        suite: PathBuf,
        #[arg(long)]
        run_id: Option<String>,
        /// Max concurrent (job, seed) pairs. Default:
        /// `min(available_parallelism(), 8)`. See `Run` for memory
        /// implications.
        #[arg(long, short = 'P')]
        parallelism: Option<usize>,
    },
}

/// Resolve user-supplied parallelism with the default cap.
///
/// `Some(n)` with `n >= 1` is returned as-is; `Some(0)` is treated as
/// "use the default". The default is `min(available_parallelism(), 8)`
/// — the cap of 8 keeps peak RSS comfortably under 32 GB on the dev
/// machine even with a 100-node topology, while saturating most
/// consumer core counts.
fn resolve_parallelism(opt: Option<usize>) -> usize {
    if let Some(n) = opt
        && n >= 1
    {
        return n;
    }
    let nproc = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    nproc.min(8)
}

fn main() -> Result<()> {
    let fmt_layer = tracing_subscriber::fmt::layer().compact().without_time();
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();
    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(filter)
        .init();

    let args = Args::parse();
    match args.command {
        Command::Run {
            suite,
            run_id,
            parallelism,
        } => run_suite_with_run_id(&suite, run_id.as_deref(), resolve_parallelism(parallelism)),
        Command::Status { suite, run_id } => print_status(&suite, run_id.as_deref()),
        Command::Verify {
            suite,
            run_id,
            parallelism,
        } => verify_suite_with_run_id(&suite, run_id.as_deref(), resolve_parallelism(parallelism)),
    }
}

fn print_status(suite_path: &std::path::Path, run_id: Option<&str>) -> Result<()> {
    let mut suite = Suite::load(suite_path)?;
    apply_run_id(&mut suite, run_id);
    let manifest_path = suite.output_dir.join("manifest.json");
    if !manifest_path.exists() {
        println!(
            "(no manifest at {} — suite has not been run)",
            manifest_path.display()
        );
        return Ok(());
    }
    let manifest = Manifest::load_or_init(&manifest_path, &suite)?;
    println!("Suite: {}", manifest.suite_name);
    println!("Started: {}", manifest.started_at_utc);
    for (job, seeds) in &manifest.jobs {
        for (seed, entry) in seeds {
            println!(
                "  job={} seed={} status={:?} output={:?}",
                job, seed, entry.status, entry.output_path
            );
        }
    }
    Ok(())
}
