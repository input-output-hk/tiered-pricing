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
    runner::{Manifest, run_suite, verify_suite},
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
    },
    /// Print the manifest's per-job status without running.
    Status {
        suite: PathBuf,
    },
    /// Re-run every Completed (job, seed) and assert each
    /// freshly-computed pricing-event-stream SHA256 matches the
    /// persisted value. Plan §M3 verification line 321: the
    /// inline determinism check inside the runner.
    Verify {
        suite: PathBuf,
    },
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
        Command::Run { suite } => run_suite(&suite),
        Command::Status { suite } => print_status(&suite),
        Command::Verify { suite } => verify_suite(&suite),
    }
}

fn print_status(suite_path: &std::path::Path) -> Result<()> {
    let suite = Suite::load(suite_path)?;
    let manifest_path = suite.output_dir.join("manifest.json");
    if !manifest_path.exists() {
        println!("(no manifest at {} — suite has not been run)", manifest_path.display());
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
