use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::process::{Child, Command};
use tokio::task::JoinSet;
use uuid::Uuid;

use crate::cgroup::{current_cgroup_path, prepare_delegated_parent, SessionCgroup};
use crate::chrome::{ChromeConfig, ChromeProcess};
use crate::event::{EventKind, EventWriter};
use crate::policy::Mechanism;
use crate::protocol::{Phase, PhaseAck, PhaseRequest, SCHEMA_VERSION};

static NEXT_PORT: AtomicU16 = AtomicU16::new(9300);

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BenchConfig {
    pub name: String,
    pub chromium: PathBuf,
    #[serde(default = "default_node")]
    pub node: PathBuf,
    #[serde(default = "default_server")]
    pub server_script: PathBuf,
    #[serde(default = "default_driver")]
    pub driver_script: PathBuf,
    #[serde(default = "default_server_port")]
    pub server_port: u16,
    #[serde(default = "default_settle")]
    pub settle_ms: u64,
    pub workloads: Vec<String>,
    pub waits_ms: Vec<u64>,
    pub concurrency: Vec<usize>,
    pub repetitions: usize,
    pub mechanisms: Vec<Mechanism>,
    #[serde(default)]
    pub chrome_args: Vec<String>,
    #[serde(default)]
    pub browser_cpus: Option<String>,
}

fn default_node() -> PathBuf {
    "node".into()
}
fn default_server() -> PathBuf {
    "workloads/server.mjs".into()
}
fn default_driver() -> PathBuf {
    "workloads/driver/driver.mjs".into()
}
fn default_server_port() -> u16 {
    4173
}
fn default_settle() -> u64 {
    3_000
}

#[derive(Debug, Deserialize)]
struct DriverResult {
    success: bool,
    latency_ms: f64,
    resume_latency_ms: f64,
    reconnects: u64,
    sequence_gaps: u64,
    #[serde(default)]
    failure: Option<String>,
}

struct CompletedTask {
    session_id: String,
    mechanism: Mechanism,
    workload: String,
    wait_ms: u64,
    result: DriverResult,
    browser_cpu_usec: u64,
    driver_cpu_usec: u64,
}

#[derive(Debug, Serialize)]
struct Environment {
    schema_version: u16,
    baeld_version: String,
    created_unix_ms: u128,
    config: BenchConfig,
    os: String,
    arch: String,
    os_release: String,
    kernel: String,
    hostname: String,
    virtualization: String,
    cpu_model: String,
    logical_cpus: usize,
    memory_total_kib: u64,
    clocksource: String,
    kernel_command_line: String,
    host_steal_ticks_start: u64,
    chromium_version: String,
    node_version: String,
    rust_version: String,
    playwright_version: String,
    git_sha: String,
    git_dirty: bool,
}

pub async fn run_smoke(output: &Path) -> Result<()> {
    run_config_impl(Path::new("experiments/smoke.toml"), output, true).await
}

pub async fn run_config(config_path: &Path, output: &Path) -> Result<()> {
    run_config_impl(config_path, output, false).await
}

async fn run_config_impl(
    config_path: &Path,
    output: &Path,
    require_all_successful: bool,
) -> Result<()> {
    if !cfg!(target_os = "linux") {
        bail!("Baeld benchmarks require Linux with cgroup v2; use Windows only for development");
    }
    let raw = fs::read_to_string(config_path)
        .with_context(|| format!("reading {}", config_path.display()))?;
    let config: BenchConfig =
        toml::from_str(&raw).with_context(|| format!("parsing {}", config_path.display()))?;
    validate_config(&config)?;

    let run_id = format!(
        "{}-{}-{}",
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        sanitize(&config.name),
        &Uuid::new_v4().to_string()[..8]
    );
    let run_dir = output.join(run_id);
    fs::create_dir_all(&run_dir)?;
    let writer = EventWriter::create(&run_dir.join("events.jsonl"))?;
    write_environment(&run_dir, &config)?;

    let parent = prepare_delegated_parent(&current_cgroup_path()?)?;
    verify_delegation(&parent)?;
    let server_cgroup = SessionCgroup::create(&parent, "workload-server")?;
    let mut server = start_server(&config, &server_cgroup).await?;
    wait_port(config.server_port, Duration::from_secs(10)).await?;

    let mut mechanisms = config.mechanisms.clone();
    mechanisms.shuffle(&mut rand::thread_rng());
    let result = run_matrix(&config, &parent, &server_cgroup, &writer, mechanisms).await;
    let _ = server.kill().await;
    let _ = server.wait().await;
    let _ = server_cgroup.remove();
    let all_successful = result?;

    println!("Results: {}", run_dir.display());
    crate::summarize::run(&run_dir, false)?;
    if require_all_successful && !all_successful {
        bail!(
            "smoke benchmark had one or more failed tasks; refusing to start a larger experiment"
        );
    }
    Ok(())
}

async fn run_matrix(
    config: &BenchConfig,
    parent: &Path,
    server_cgroup: &SessionCgroup,
    writer: &EventWriter,
    mechanisms: Vec<Mechanism>,
) -> Result<bool> {
    let mut all_successful = true;
    for workload in &config.workloads {
        for &wait_ms in &config.waits_ms {
            for &concurrency in &config.concurrency {
                for _ in 0..config.repetitions {
                    let mut block = mechanisms.clone();
                    block.shuffle(&mut rand::thread_rng());
                    for mechanism in block {
                        all_successful &= run_group(
                            config,
                            parent,
                            server_cgroup,
                            writer,
                            &mechanism,
                            workload,
                            wait_ms,
                            concurrency,
                        )
                        .await?;
                    }
                }
            }
        }
    }
    Ok(all_successful)
}

#[allow(clippy::too_many_arguments)]
async fn run_group(
    config: &BenchConfig,
    parent: &Path,
    server_cgroup: &SessionCgroup,
    writer: &EventWriter,
    mechanism: &Mechanism,
    workload: &str,
    wait_ms: u64,
    concurrency: usize,
) -> Result<bool> {
    let governor_before = process_cpu_usec();
    let server_before = server_cgroup.sample()?;
    let steal_before = host_steal_ticks();
    let mut tasks = JoinSet::new();
    for _ in 0..concurrency {
        let config = config.clone();
        let parent = parent.to_owned();
        let writer = writer.clone();
        let mechanism = mechanism.clone();
        let workload = workload.to_owned();
        tasks.spawn(async move {
            run_one(&config, &parent, &writer, mechanism, &workload, wait_ms).await
        });
    }
    let mut completed = Vec::with_capacity(concurrency);
    while let Some(result) = tasks.join_next().await {
        completed.push(result.context("benchmark task panicked")??);
    }
    let governor_total = process_cpu_usec().saturating_sub(governor_before);
    let server_total = server_cgroup.sample()?.delta(&server_before).cpu_usage_usec;
    let steal_total = host_steal_ticks().saturating_sub(steal_before);
    let governor_share = governor_total / completed.len().max(1) as u64;
    let server_share = server_total / completed.len().max(1) as u64;
    let steal_share = steal_total / completed.len().max(1) as u64;
    let all_successful = completed.iter().all(|task| task.result.success);
    for task in completed {
        writer.write(
            &task.session_id,
            u64::MAX,
            Phase::Finished,
            &task.mechanism,
            EventKind::TaskFinished {
                workload: task.workload,
                wait_ms: task.wait_ms,
                success: task.result.success,
                latency_ms: task.result.latency_ms,
                browser_cpu_usec: task.browser_cpu_usec,
                driver_cpu_usec: task.driver_cpu_usec,
                governor_cpu_usec: governor_share,
                server_cpu_usec: server_share,
                host_steal_ticks: steal_share,
                resume_latency_ms: task.result.resume_latency_ms,
                reconnects: task.result.reconnects,
                sequence_gaps: task.result.sequence_gaps,
                failure: task.result.failure,
            },
        )?;
    }
    Ok(all_successful)
}

async fn run_one(
    config: &BenchConfig,
    parent: &Path,
    writer: &EventWriter,
    mechanism: Mechanism,
    workload: &str,
    wait_ms: u64,
) -> Result<CompletedTask> {
    let session_id = Uuid::new_v4().to_string();
    let cgroup = Arc::new(SessionCgroup::create(parent, &session_id)?);
    let port = NEXT_PORT.fetch_add(1, Ordering::Relaxed);
    let chrome_config = ChromeConfig {
        executable: config.chromium.clone(),
        remote_debugging_host: "127.0.0.1".into(),
        extra_args: config.chrome_args.clone(),
        cpu_affinity: config.browser_cpus.clone(),
    };
    let mut chrome = ChromeProcess::launch(&chrome_config, &cgroup, port).await?;
    if cgroup.pids()?.is_empty() {
        bail!(
            "Chromium process tree did not enter {}",
            cgroup.path().display()
        );
    }

    let socket_dir = tempfile::Builder::new().prefix("baeld-socket-").tempdir()?;
    let socket_path = socket_dir.path().join("phase.sock");
    let listener = UnixListener::bind(&socket_path)?;
    let controller_cgroup = cgroup.clone();
    let controller_writer = writer.clone();
    let controller_mechanism = mechanism.clone();
    let controller_session = session_id.clone();
    let controller = tokio::spawn(async move {
        serve_phase_protocol(
            listener,
            controller_cgroup,
            controller_writer,
            controller_mechanism,
            controller_session,
        )
        .await
    });

    let before = cgroup.sample()?;
    let sampler_cgroup = cgroup.clone();
    let sampler_writer = writer.clone();
    let sampler_session = session_id.clone();
    let sampler_mechanism = mechanism.clone();
    let sampler = tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(500)).await;
            if let Ok(sample) = sampler_cgroup.sample() {
                let _ = sampler_writer.write(
                    &sampler_session,
                    0,
                    Phase::Starting,
                    &sampler_mechanism,
                    EventKind::ResourceSample(sample),
                );
            }
        }
    });
    let driver_cgroup = SessionCgroup::create(parent, &format!("driver-{session_id}"))?;
    let driver_before = driver_cgroup.sample()?;
    let started = Instant::now();
    let driver_script = format!(
        "{} {} {}",
        driver_cgroup.join_command_prefix(),
        shell_word(&config.node.to_string_lossy()),
        shell_word(&config.driver_script.to_string_lossy())
    );
    let mut driver_command = Command::new("/bin/sh");
    driver_command
        .args(["-c", &driver_script])
        .env("BAELD_SESSION_ID", &session_id)
        .env("BAELD_SOCKET", &socket_path)
        .env("BAELD_CDP_URL", chrome.websocket_discovery_url())
        .env(
            "BAELD_BASE_URL",
            format!("http://127.0.0.1:{}", config.server_port),
        )
        .env("BAELD_WORKLOAD", workload)
        .env("BAELD_WAIT_MS", wait_ms.to_string())
        .env("BAELD_SETTLE_MS", config.settle_ms.to_string())
        .env("BAELD_MECHANISM", mechanism.slug())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let driver = driver_command
        .spawn()
        .context("starting Playwright driver")?;
    // A broken browser, driver, or phase exchange must not consume a paid
    // benchmark host indefinitely. The allowance is deliberately generous
    // relative to the configured wait and settling windows.
    let driver_timeout = Duration::from_millis(
        wait_ms
            .saturating_add(config.settle_ms)
            .saturating_add(60_000),
    );
    let output = tokio::time::timeout(driver_timeout, driver.wait_with_output()).await;
    let latency = started.elapsed();
    if output.is_err() {
        // Dropping Child::wait_with_output kills the driver because
        // kill_on_drop is enabled. Thaw first so cleanup can always proceed.
        let _ = cgroup.thaw();
        let _ = cgroup.set_cpu_max(None, 100_000);
        let _ = driver_cgroup.kill_all();
    }
    let driver_delta = driver_cgroup.sample()?.delta(&driver_before);
    let after = cgroup.sample()?;
    sampler.abort();
    let _ = sampler.await;
    match tokio::time::timeout(Duration::from_secs(2), controller).await {
        Ok(joined) => joined.context("phase controller panicked")??,
        Err(_) => bail!("phase controller did not stop after finished phase"),
    }

    let parsed = match output {
        Ok(Ok(output)) if output.status.success() => {
            serde_json::from_slice::<DriverResult>(&output.stdout).unwrap_or_else(|error| {
                DriverResult {
                    success: false,
                    latency_ms: latency.as_secs_f64() * 1_000.0,
                    resume_latency_ms: 0.0,
                    reconnects: 0,
                    sequence_gaps: 0,
                    failure: Some(format!(
                        "invalid driver output: {error}; stdout={:?}; stderr={:?}",
                        String::from_utf8_lossy(&output.stdout).trim(),
                        String::from_utf8_lossy(&output.stderr).trim()
                    )),
                }
            })
        }
        Ok(Ok(output)) => DriverResult {
            success: false,
            latency_ms: latency.as_secs_f64() * 1_000.0,
            resume_latency_ms: 0.0,
            reconnects: 0,
            sequence_gaps: 0,
            failure: Some(format!(
                "driver exited with {}; stderr={:?}",
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            )),
        },
        Ok(Err(error)) => DriverResult {
            success: false,
            latency_ms: latency.as_secs_f64() * 1_000.0,
            resume_latency_ms: 0.0,
            reconnects: 0,
            sequence_gaps: 0,
            failure: Some(format!("waiting for Playwright driver: {error}")),
        },
        Err(_) => DriverResult {
            success: false,
            latency_ms: latency.as_secs_f64() * 1_000.0,
            resume_latency_ms: 0.0,
            reconnects: 0,
            sequence_gaps: 0,
            failure: Some(format!("Playwright driver exceeded {driver_timeout:?}")),
        },
    };

    let delta = after.delta(&before);

    chrome.terminate().await;
    drop(chrome);
    driver_cgroup.remove()?;
    Arc::try_unwrap(cgroup)
        .map_err(|_| anyhow::anyhow!("session cgroup still has outstanding owners"))?
        .remove()?;
    Ok(CompletedTask {
        session_id,
        mechanism,
        workload: workload.to_owned(),
        wait_ms,
        result: parsed,
        browser_cpu_usec: delta.cpu_usage_usec,
        driver_cpu_usec: driver_delta.cpu_usage_usec,
    })
}

async fn serve_phase_protocol(
    listener: UnixListener,
    cgroup: Arc<SessionCgroup>,
    writer: EventWriter,
    mechanism: Mechanism,
    session_id: String,
) -> Result<()> {
    let (stream, _) = listener.accept().await?;
    let (reader, mut output) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();
    let mut last_generation = 0;
    let mut pending_freeze: Option<tokio::task::JoinHandle<()>> = None;

    while let Some(line) = lines.next_line().await? {
        let request: PhaseRequest = serde_json::from_str(&line)?;
        let response = match request.validate(&session_id, last_generation) {
            Err(error) => PhaseAck {
                schema_version: SCHEMA_VERSION,
                session_id: session_id.clone(),
                generation: request.generation,
                accepted: false,
                active: !cgroup.is_frozen().unwrap_or(false),
                error: Some(error),
            },
            Ok(()) => {
                last_generation = request.generation;
                if request.phase != Phase::WaitingForModel {
                    if let Some(task) = pending_freeze.take() {
                        task.abort();
                        let _ = task.await;
                    }
                    let _ = cgroup.thaw();
                    let _ = cgroup.set_cpu_max(None, 100_000);
                }
                let applied = apply_wait_policy(
                    request.phase,
                    &mechanism,
                    cgroup.clone(),
                    &mut pending_freeze,
                );
                let error = applied.err().map(|error| error.to_string());
                if error.is_some() {
                    let _ = cgroup.thaw();
                    let _ = cgroup.set_cpu_max(None, 100_000);
                }
                writer.write(
                    &session_id,
                    request.generation,
                    request.phase,
                    &mechanism,
                    EventKind::PolicyApplied {
                        action: if request.phase == Phase::WaitingForModel {
                            mechanism.slug()
                        } else {
                            "restore-unrestricted".into()
                        },
                    },
                )?;
                writer.write(
                    &session_id,
                    request.generation,
                    request.phase,
                    &mechanism,
                    EventKind::PhaseChanged,
                )?;
                PhaseAck {
                    schema_version: SCHEMA_VERSION,
                    session_id: session_id.clone(),
                    generation: request.generation,
                    accepted: error.is_none(),
                    active: !cgroup.is_frozen().unwrap_or(false),
                    error,
                }
            }
        };
        output
            .write_all(serde_json::to_string(&response)?.as_bytes())
            .await?;
        output.write_all(b"\n").await?;
        if request.phase == Phase::Finished {
            if let Some(task) = pending_freeze.take() {
                task.abort();
                let _ = task.await;
            }
            let _ = cgroup.thaw();
            let _ = cgroup.set_cpu_max(None, 100_000);
            break;
        }
    }
    Ok(())
}

fn apply_wait_policy(
    phase: Phase,
    mechanism: &Mechanism,
    cgroup: Arc<SessionCgroup>,
    pending_freeze: &mut Option<tokio::task::JoinHandle<()>>,
) -> Result<()> {
    if phase != Phase::WaitingForModel {
        return Ok(());
    }
    match mechanism {
        Mechanism::CpuQuota {
            quota_us,
            period_us,
        } => cgroup.set_cpu_max(Some(*quota_us), *period_us),
        Mechanism::CgroupFreeze { delay_ms } => {
            let delay = Duration::from_millis(*delay_ms);
            *pending_freeze = Some(tokio::spawn(async move {
                tokio::time::sleep(delay).await;
                let _ = cgroup.freeze();
            }));
            Ok(())
        }
        Mechanism::Baseline | Mechanism::ChromeLifecycleFreeze => Ok(()),
    }
}

fn validate_config(config: &BenchConfig) -> Result<()> {
    if config.workloads.is_empty()
        || config.waits_ms.is_empty()
        || config.concurrency.is_empty()
        || config.mechanisms.is_empty()
        || config.repetitions == 0
    {
        bail!("benchmark matrix fields must be non-empty and repetitions > 0");
    }
    if config.concurrency.iter().any(|&value| value == 0) {
        bail!("concurrency must be positive");
    }
    for mechanism in &config.mechanisms {
        mechanism.validate()?;
    }
    Ok(())
}

async fn start_server(config: &BenchConfig, cgroup: &SessionCgroup) -> Result<Child> {
    let script = format!(
        "{} {} {}",
        cgroup.join_command_prefix(),
        shell_word(&config.node.to_string_lossy()),
        shell_word(&config.server_script.to_string_lossy())
    );
    let mut command = Command::new("/bin/sh");
    command
        .args(["-c", &script])
        .env("PORT", config.server_port.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    command.spawn().context("starting workload server")
}

async fn wait_port(port: u16, timeout: Duration) -> Result<()> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .is_ok()
        {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    bail!("workload server did not listen on port {port}")
}

fn verify_delegation(parent: &Path) -> Result<()> {
    for controller in ["cpu", "memory", "io", "pids"] {
        let available = fs::read_to_string(parent.join("cgroup.subtree_control"))?;
        if !available
            .split_whitespace()
            .any(|value| value == controller)
        {
            bail!(
                "controller {controller} is not enabled at {}",
                parent.display()
            );
        }
    }
    Ok(())
}

fn write_environment(run_dir: &Path, config: &BenchConfig) -> Result<()> {
    let environment = Environment {
        schema_version: 1,
        baeld_version: env!("CARGO_PKG_VERSION").into(),
        created_unix_ms: SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis(),
        config: config.clone(),
        os: std::env::consts::OS.into(),
        arch: std::env::consts::ARCH.into(),
        os_release: read_trimmed("/etc/os-release"),
        kernel: command_line("uname", &["-a"]),
        hostname: command_line("hostname", &[]),
        virtualization: command_line("systemd-detect-virt", &[]),
        cpu_model: cpu_model(),
        logical_cpus: std::thread::available_parallelism()
            .map(usize::from)
            .unwrap_or_default(),
        memory_total_kib: memory_total_kib(),
        clocksource: read_trimmed(
            "/sys/devices/system/clocksource/clocksource0/current_clocksource",
        ),
        kernel_command_line: read_trimmed("/proc/cmdline"),
        host_steal_ticks_start: host_steal_ticks(),
        chromium_version: command_line(config.chromium.to_string_lossy().as_ref(), &["--version"]),
        node_version: command_line(config.node.to_string_lossy().as_ref(), &["--version"]),
        rust_version: command_line("rustc", &["--version", "--verbose"]),
        playwright_version: command_line(
            config.node.to_string_lossy().as_ref(),
            &[
                "-p",
                "require('./node_modules/playwright/package.json').version",
            ],
        ),
        git_sha: command_line("git", &["rev-parse", "HEAD"]),
        git_dirty: !command_line("git", &["status", "--porcelain"]).is_empty(),
    };
    fs::write(
        run_dir.join("environment.json"),
        serde_json::to_vec_pretty(&environment)?,
    )?;
    Ok(())
}

fn read_trimmed(path: &str) -> String {
    fs::read_to_string(path)
        .map(|value| value.trim().to_owned())
        .unwrap_or_else(|error| format!("unavailable: {error}"))
}

fn cpu_model() -> String {
    fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|cpuinfo| {
            cpuinfo.lines().find_map(|line| {
                line.strip_prefix("model name")
                    .and_then(|value| value.split_once(':'))
                    .map(|(_, value)| value.trim().to_owned())
            })
        })
        .unwrap_or_else(|| "unavailable".into())
}

fn memory_total_kib() -> u64 {
    fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|meminfo| {
            meminfo.lines().find_map(|line| {
                line.strip_prefix("MemTotal:")?
                    .split_whitespace()
                    .next()?
                    .parse()
                    .ok()
            })
        })
        .unwrap_or_default()
}

fn command_line(command: &str, args: &[&str]) -> String {
    std::process::Command::new(command)
        .args(args)
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .unwrap_or_else(|error| format!("unavailable: {error}"))
}

fn process_cpu_usec() -> u64 {
    let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
    if unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut usage) } != 0 {
        return 0;
    }
    let user = usage.ru_utime.tv_sec as u64 * 1_000_000 + usage.ru_utime.tv_usec as u64;
    let system = usage.ru_stime.tv_sec as u64 * 1_000_000 + usage.ru_stime.tv_usec as u64;
    user + system
}

fn host_steal_ticks() -> u64 {
    fs::read_to_string("/proc/stat")
        .ok()
        .and_then(|value| {
            value
                .lines()
                .find(|line| line.starts_with("cpu "))
                .and_then(|line| line.split_whitespace().nth(8))
                .and_then(|value| value.parse().ok())
        })
        .unwrap_or_default()
}

fn sanitize(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect()
}

fn shell_word(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}
