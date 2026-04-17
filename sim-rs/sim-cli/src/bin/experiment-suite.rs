use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use sim_cli::{runner, suite};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Parser)]
#[command(version = concat!(env!("CARGO_PKG_VERSION"), "-", env!("VERGEN_GIT_SHA")))]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Run {
        suite: PathBuf,
        #[clap(long)]
        label: Option<String>,
        #[clap(long = "output-root")]
        output_root: Option<PathBuf>,
    },
    Resume {
        suite_run_dir: PathBuf,
    },
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    runner::init_tracing();

    let args = Args::parse();
    let token = CancellationToken::new();
    runner::install_ctrlc_handler(token.clone())?;

    match args.command {
        Command::Run {
            suite: suite_path,
            label,
            output_root,
        } => {
            let _run_dir =
                suite::run_suite(suite_path, label.as_deref(), output_root.as_deref(), token)
                    .await?;
        }
        Command::Resume { suite_run_dir } => {
            suite::resume_suite(suite_run_dir, token).await?;
        }
    }

    Ok(())
}
