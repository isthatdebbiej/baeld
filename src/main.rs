#[cfg(target_os = "linux")]
mod bench;
#[cfg(target_os = "linux")]
mod cgroup;
#[cfg(target_os = "linux")]
mod chrome;
mod config;
mod doctor;
mod event;
mod health;
mod metrics;
mod policy;
mod protocol;
#[cfg(target_os = "linux")]
mod runtime;
#[cfg(target_os = "linux")]
mod scheduler;
mod summarize;
#[cfg(target_os = "linux")]
mod telemetry;

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
#[command(
    name = "baeld",
    version,
    about = "Browser-agent performance runtime and benchmark"
)]
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
    /// Run and supervise an agent connected to a Baeld-owned Chromium session.
    Run {
        #[arg(long, default_value = "baeld.toml")]
        config: PathBuf,
        #[arg(required = true, trailing_var_arg = true)]
        command: Vec<String>,
    },
    /// Inspect a live or recently completed runtime session.
    Inspect {
        session_id: String,
        #[arg(long)]
        json: bool,
    },
    /// List runtime sessions known to this machine.
    Sessions {
        #[arg(long)]
        json: bool,
    },
    /// Remove stale session metadata and abandoned owned resources.
    Cleanup,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Doctor { json } => doctor::run(json).await,
        Command::Smoke { output } => bench::run_smoke(&output).await,
        Command::Bench { config, output } => bench::run_config(&config, &output).await,
        Command::Summarize { run, json } => summarize::run(&run, json),
        #[cfg(target_os = "linux")]
        Command::Run { config, command } => runtime::run(&config, &command).await,
        #[cfg(not(target_os = "linux"))]
        Command::Run { .. } => anyhow::bail!("Baeld runtime requires Linux with cgroup v2"),
        #[cfg(target_os = "linux")]
        Command::Inspect { session_id, json } => runtime::inspect(&session_id, json),
        #[cfg(not(target_os = "linux"))]
        Command::Inspect { .. } => anyhow::bail!("Baeld runtime requires Linux with cgroup v2"),
        #[cfg(target_os = "linux")]
        Command::Sessions { json } => runtime::sessions(json),
        #[cfg(not(target_os = "linux"))]
        Command::Sessions { .. } => anyhow::bail!("Baeld runtime requires Linux with cgroup v2"),
        #[cfg(target_os = "linux")]
        Command::Cleanup => runtime::cleanup(),
        #[cfg(not(target_os = "linux"))]
        Command::Cleanup => anyhow::bail!("Baeld runtime requires Linux with cgroup v2"),
    }
}
