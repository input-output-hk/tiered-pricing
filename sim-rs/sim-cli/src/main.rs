use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use sim_cli::runner::{self, RunRequest};
use tokio_util::sync::CancellationToken;

#[derive(Parser)]
#[command(version = concat!(env!("CARGO_PKG_VERSION"), "-", env!("VERGEN_GIT_SHA")))]
struct Args {
    #[clap(default_value = None)]
    topology: Option<PathBuf>,
    output: Option<PathBuf>,
    #[clap(short, long)]
    parameters: Vec<PathBuf>,
    #[clap(long = "compare-parameters")]
    compare_parameters: Vec<PathBuf>,
    #[clap(long = "comparison-output")]
    comparison_output: Option<PathBuf>,
    #[clap(short, long)]
    timescale: Option<f64>,
    #[clap(long)]
    trace_node: Vec<usize>,
    #[clap(short, long)]
    slots: Option<u64>,
    #[clap(short, long)]
    conformance_events: bool,
    #[clap(short, long)]
    aggregate_events: bool,
    #[clap(long)]
    no_trace: bool,
}

impl From<Args> for RunRequest {
    fn from(args: Args) -> Self {
        Self {
            topology: args.topology,
            output: args.output,
            parameters: args.parameters,
            compare_parameters: args.compare_parameters,
            comparison_output: args.comparison_output,
            timescale: args.timescale,
            trace_nodes: args.trace_node,
            slots: args.slots,
            conformance_events: args.conformance_events,
            aggregate_events: args.aggregate_events,
            no_trace: args.no_trace,
            trailing_parameters: Vec::new(),
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    runner::init_tracing();

    let request = RunRequest::from(Args::parse());
    let token = CancellationToken::new();
    runner::install_ctrlc_handler(token.clone())?;
    let _outcome = runner::execute_run_request(&request, token).await?;
    Ok(())
}
