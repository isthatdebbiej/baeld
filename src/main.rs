#[cfg(target_os = "linux")]
mod bench;
#[cfg(target_os = "linux")]
mod cgroup;
#[cfg(target_os = "linux")]
mod chrome;
mod doctor;
mod event;
mod metrics;
mod policy;
mod protocol;
mod summarize;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[cfg(not(target_os = "linux"))]
mod bench {
    use anyhow::{bail, Result};
    use std::path::Path;

    pub async fn run_smoke(_output: &Path) -> Result<()> {
        bail!("Baeld benchmarks require Linux with cgroup v2")
    }

    pub async fn run_config(_config: &Path, _output: &Path) -> Result<()> {
        bail!("Baeld benchmarks require Linux with cgroup v2")
    }
}

#[derive(Debug, Parser)]
#[command(name = "baeld", version, about = "Browser-agent suspension benchmark")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Verify that the Linux host can run publishable Baeld experiments.
    Doctor {
        /// Emit machine-readable JSON instead of a table.
        #[arg(long)]
        json: bool,
    },
    /// Run all four mechanisms and retain application-policy failures.
    Smoke {
        /// Directory in which to create the immutable result run.
        #[arg(long, default_value = "results")]
        output: PathBuf,
    },
    /// Run an experiment described by a TOML configuration.
    Bench {
        #[arg(long)]
        config: PathBuf,
        #[arg(long, default_value = "results")]
        output: PathBuf,
    },
    /// Summarize an existing result directory.
    Summarize {
        run: PathBuf,
        #[arg(long)]
        json: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Doctor { json } => doctor::run(json).await,
        Command::Smoke { output } => bench::run_smoke(&output).await,
        Command::Bench { config, output } => bench::run_config(&config, &output).await,
        Command::Summarize { run, json } => summarize::run(&run, json),
    }
}
