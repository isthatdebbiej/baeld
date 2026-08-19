use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::process::Command;
use uuid::Uuid;

use crate::cgroup::{current_cgroup_path, prepare_delegated_parent, SessionCgroup};
use crate::chrome::{ChromeConfig, ChromeProcess};
use crate::config::{FreezeCompatibility, PolicyMode, RuntimeConfig, SessionMode};
use crate::health::{HealthMonitor, HealthState};
use crate::policy::Mechanism;
use crate::protocol::{valid_transition, Phase, PhaseAck, PhaseRequest, SCHEMA_VERSION};
use crate::scheduler::{AdmissionController, Permit};
use crate::telemetry::{RuntimeEvent, Telemetry};

#[derive(Debug, Deserialize)]
struct CompatibilityFile {
    records: Vec<CompatibilityRecord>,
}

#[derive(Debug, Deserialize)]
struct CompatibilityRecord {
    framework: String,
    workload: String,
    browser_version: String,
    mechanism: String,
    compatible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionRecord {
    schema_version: u16,
    session_id: String,
    runtime_pid: u32,
    command: String,
    state: HealthState,
    created_unix_ms: u128,
    updated_unix_ms: u128,
    cgroup: PathBuf,
    socket: PathBuf,
    cdp_url: String,
    profile: PathBuf,
    policy_mode: PolicyMode,
    session_mode: SessionMode,
    adapter_connected: bool,
    message: Option<String>,
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|v| v.as_millis())
        .unwrap_or(0)
}

fn state_root() -> PathBuf {
    std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))
        .unwrap_or_else(|| PathBuf::from(".baeld/state"))
        .join("baeld")
}

fn record_path(id: &str) -> PathBuf {
    state_root().join("sessions").join(format!("{id}.json"))
}

fn write_record(record: &SessionRecord) -> Result<()> {
    let path = record_path(&record.session_id);
    secure_directory(path.parent().expect("record parent"))?;
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, serde_json::to_vec_pretty(record)?)?;
    fs::rename(temporary, path)?;
    Ok(())
}

fn secure_directory(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::create_dir_all(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

fn read_records() -> Result<Vec<SessionRecord>> {
    let directory = state_root().join("sessions");
    if !directory.exists() {
        return Ok(vec![]);
    }
    let mut records = Vec::new();
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.extension().and_then(|v| v.to_str()) != Some("json") {
            continue;
        }
        if let Ok(record) = serde_json::from_slice(&fs::read(path)?) {
            records.push(record);
        }
    }
    records.sort_by_key(|record: &SessionRecord| record.created_unix_ms);
    Ok(records)
}

fn process_alive(pid: u32) -> bool {
    Path::new(&format!("/proc/{pid}")).exists()
}

pub fn inspect(session_id: &str, json: bool) -> Result<()> {
    let record: SessionRecord = serde_json::from_slice(
        &fs::read(record_path(session_id))
            .with_context(|| format!("session {session_id} not found"))?,
    )?;
    if json {
        println!("{}", serde_json::to_string_pretty(&record)?);
    } else {
        println!(
            "{}\t{:?}\t{}\t{}",
            record.session_id,
            record.state,
            if process_alive(record.runtime_pid) {
                "live"
            } else {
                "ended"
            },
            record.cdp_url
        );
    }
    Ok(())
}

pub fn sessions(json: bool) -> Result<()> {
    let records = read_records()?;
    if json {
        println!("{}", serde_json::to_string_pretty(&records)?);
    } else {
        println!("SESSION\tSTATE\tRUNTIME\tPOLICY");
        for record in records {
            println!(
                "{}\t{:?}\t{}\t{:?}",
                record.session_id,
                record.state,
                if process_alive(record.runtime_pid) {
                    "live"
                } else {
                    "ended"
                },
                record.policy_mode
            );
        }
    }
    Ok(())
}

pub fn cleanup() -> Result<()> {
    let mut removed = 0;
    for record in read_records()? {
        if process_alive(record.runtime_pid) {
            continue;
        }
        let expected_name = format!("session-{}", record.session_id);
        let valid_cgroup = record.cgroup.starts_with("/sys/fs/cgroup")
            && record.cgroup.file_name().and_then(|value| value.to_str())
                == Some(expected_name.as_str());
        if valid_cgroup {
            if let Ok(cgroup) = SessionCgroup::open_existing(record.cgroup.clone()) {
                let _ = cgroup.remove();
            }
        } else {
            eprintln!(
                "Refused unexpected cgroup path in session {}",
                record.session_id
            );
        }
        let _ = fs::remove_file(&record.socket);
        let disposable_profile = record.session_mode != SessionMode::Persistent
            && record
                .profile
                .file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.starts_with("baeld-chrome-"))
            && record.profile.parent() == Some(std::env::temp_dir().as_path());
        if disposable_profile {
            let _ = fs::remove_dir_all(&record.profile);
        }
        let _ = fs::remove_file(record_path(&record.session_id));
        removed += 1;
    }
    println!("Removed {removed} stale session record(s)");
    Ok(())
}

pub async fn run(config_path: &Path, command: &[String]) -> Result<()> {
    if command.is_empty() {
        bail!("a command is required after --");
    }
    let config = RuntimeConfig::load(config_path)?;
    let id = Uuid::new_v4().to_string();
    let root = state_root();
    let session_dir = root.join("runtime").join(&id);
    secure_directory(&session_dir)?;
    let socket = session_dir.join("phase.sock");
    let _ = fs::remove_file(&socket);
    let listener = UnixListener::bind(&socket)?;
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&socket, fs::Permissions::from_mode(0o600))?;
    }
    let telemetry = Telemetry::new(
        &session_dir.join("events.jsonl"),
        config.telemetry.jsonl,
        &config.telemetry.otel_endpoint,
    )?;

    let parent = prepare_delegated_parent(&current_cgroup_path()?)?;
    let cgroup = Arc::new(SessionCgroup::create(&parent, &id)?);
    cgroup.set_memory_max(config.session.max_memory_mb * 1024 * 1024)?;
    cgroup.set_memory_high(config.session.max_memory_mb * 1024 * 1024 * 9 / 10)?;
    cgroup.set_pids_max(config.session.max_processes)?;
    let mut watchdog = spawn_watchdog(std::process::id(), cgroup.path())?;

    let profile = match config.session.mode {
        SessionMode::Persistent => config.session.profile_dir.clone(),
        SessionMode::Ephemeral | SessionMode::Warm => None,
    };
    let port = available_port()?;
    let mut chrome = ChromeProcess::launch(
        &ChromeConfig {
            executable: config.chromium.clone(),
            remote_debugging_host: "127.0.0.1".into(),
            extra_args: filter_chrome_args(&config),
            allow_extensions: config.allow_extensions,
            cpu_affinity: None,
            profile_dir: profile,
        },
        &cgroup,
        port,
    )
    .await?;
    let cdp_url = chrome.websocket_discovery_url();
    let chrome_pid = chrome
        .root_pid()
        .context("Chromium exited before ownership verification")?;
    verify_process_tree(chrome_pid, &cgroup)?;
    let mut record = SessionRecord {
        schema_version: 1,
        session_id: id.clone(),
        runtime_pid: std::process::id(),
        command: command[0].clone(),
        state: HealthState::Starting,
        created_unix_ms: now_ms(),
        updated_unix_ms: now_ms(),
        cgroup: cgroup.path().to_owned(),
        socket: socket.clone(),
        cdp_url: cdp_url.clone(),
        profile: chrome.profile.clone(),
        policy_mode: config.policy.mode,
        session_mode: config.session.mode,
        adapter_connected: false,
        message: None,
    };
    write_record(&record)?;

    let shared_record = Arc::new(Mutex::new(record.clone()));
    let scheduler = Arc::new(AdmissionController::new(
        &root.join("permits"),
        config.concurrency.max_active_navigation,
        config.concurrency.max_active_actions,
        Duration::from_millis(config.concurrency.stagger_resume_ms),
        config.concurrency.max_cpu_pressure_avg10,
        config.concurrency.max_memory_pressure_avg10,
    )?);
    let protocol = tokio::spawn(serve_protocol(
        listener,
        cgroup.clone(),
        chrome_pid,
        config.clone(),
        scheduler,
        telemetry.clone(),
        shared_record.clone(),
    ));

    let mut child = Command::new(&command[0]);
    child
        .args(&command[1..])
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .env("BAELD_SESSION_ID", &id)
        .env("BAELD_PHASE_SOCKET", &socket)
        .env("BAELD_CDP_URL", &cdp_url)
        .env(
            "BAELD_FILTER_PROFILE",
            format!("{:?}", config.filtering.profile).to_lowercase(),
        )
        .env(
            "BAELD_SESSION_MODE",
            format!("{:?}", config.session.mode).to_lowercase(),
        );
    let mut child = child
        .spawn()
        .with_context(|| format!("starting {}", command[0]))?;
    let mut monitor = HealthMonitor::new(
        config.session.max_memory_mb * 1024 * 1024 * 9 / 10,
        config.session.max_processes as usize,
        Duration::from_millis(config.policy.phase_timeout_ms),
    );
    let mut last_phase_update = record.updated_unix_ms;
    let mut interval = tokio::time::interval(Duration::from_millis(config.policy.sample_ms));
    let exit = loop {
        tokio::select! {
            status = child.wait() => break status?,
            _ = interval.tick() => {
                if let Ok(value) = shared_record.lock() {
                    if !value.adapter_connected || value.updated_unix_ms > last_phase_update {
                        monitor.phase_seen();
                        last_phase_update = value.updated_unix_ms;
                    }
                }
                let sample = cgroup.sample()?;
                let cdp_healthy = tokio::net::TcpStream::connect(("127.0.0.1", port)).await.is_ok();
                let health = monitor.update(sample, cdp_healthy);
                if let Ok(mut value) = shared_record.lock() { value.state = health.state; value.updated_unix_ms = now_ms(); let _ = write_record(&value); }
                let mut event = RuntimeEvent::new(&id, "health"); event.health = Some(health.clone()); telemetry.emit(event).await;
                if health.state == HealthState::Stuck
                    || (health.state == HealthState::Degraded && config.session.recycle_on_degraded)
                {
                    let _ = child.kill().await;
                }
            }
        }
    };

    protocol.abort();
    let _ = protocol.await;
    let _ = cgroup.thaw();
    let _ = cgroup.set_cpu_max(None, 100_000);
    monitor.terminating();
    chrome
        .terminate_gracefully(Duration::from_millis(config.session.shutdown_grace_ms))
        .await;
    let owned = Arc::try_unwrap(cgroup)
        .map_err(|_| anyhow::anyhow!("session controller still owns browser cgroup"))?;
    owned.remove()?;
    let _ = watchdog.kill().await;
    let _ = watchdog.wait().await;
    monitor.cleaned();
    record = shared_record
        .lock()
        .map_err(|_| anyhow::anyhow!("session registry poisoned"))?
        .clone();
    record.state = HealthState::Cleaned;
    record.updated_unix_ms = now_ms();
    record.message = Some(format!("child exited with {exit}"));
    write_record(&record)?;
    let mut event = RuntimeEvent::new(&id, "cleaned");
    event.message = record.message.clone();
    telemetry.emit(event).await;
    if exit.success() {
        Ok(())
    } else {
        bail!("agent command exited with {exit}")
    }
}

fn spawn_watchdog(parent_pid: u32, cgroup: &Path) -> Result<tokio::process::Child> {
    let kill = cgroup.join("cgroup.kill");
    let script = format!(
        "while kill -0 {parent_pid} 2>/dev/null; do sleep 1; done; if [ -e '{}' ]; then printf 1 > '{}'; fi; rmdir '{}' 2>/dev/null || true",
        kill.display(), kill.display(), cgroup.display()
    );
    Command::new("/bin/sh")
        .args(["-c", &script])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("starting controller-death watchdog")
}

fn available_port() -> Result<u16> {
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0))?;
    Ok(listener.local_addr()?.port())
}

fn filter_chrome_args(config: &RuntimeConfig) -> Vec<String> {
    use crate::config::FilterProfile::*;
    let mut args = config.chrome_args.clone();
    match config.filtering.profile {
        Safe | Visual => {}
        Balanced => args.extend([
            "--autoplay-policy=user-gesture-required".into(),
            "--disable-background-networking".into(),
        ]),
        Text => args.extend([
            "--blink-settings=imagesEnabled=false".into(),
            "--autoplay-policy=user-gesture-required".into(),
            "--disable-background-networking".into(),
            "--disable-gpu".into(),
        ]),
    }
    args
}

async fn serve_protocol(
    listener: UnixListener,
    cgroup: Arc<SessionCgroup>,
    chrome_pid: u32,
    config: RuntimeConfig,
    scheduler: Arc<AdmissionController>,
    telemetry: Telemetry,
    record: Arc<Mutex<SessionRecord>>,
) -> Result<()> {
    let latest_generation = Arc::new(AtomicU64::new(0));
    struct FailOpenGuard {
        cgroup: Arc<SessionCgroup>,
        generation: Arc<AtomicU64>,
    }
    impl Drop for FailOpenGuard {
        fn drop(&mut self) {
            self.generation.store(u64::MAX, Ordering::SeqCst);
            let _ = self.cgroup.thaw();
            let _ = self.cgroup.set_cpu_max(None, 100_000);
        }
    }
    let _fail_open = FailOpenGuard {
        cgroup: cgroup.clone(),
        generation: latest_generation.clone(),
    };
    loop {
        let (stream, _) = listener.accept().await?;
        if let Ok(mut value) = record.lock() {
            value.adapter_connected = true;
            value.updated_unix_ms = now_ms();
            let _ = write_record(&value);
        }
        let (reader, mut writer) = stream.into_split();
        let mut lines = BufReader::new(reader).lines();
        let mut previous = None;
        let mut pending: Option<tokio::task::JoinHandle<()>> = None;
        let mut permit: Option<Permit> = None;
        while let Some(line) = lines.next_line().await? {
            let request: PhaseRequest = serde_json::from_str(&line)?;
            if let Ok(mut value) = record.lock() {
                value.updated_unix_ms = now_ms();
                let _ = write_record(&value);
            }
            let last = latest_generation.load(Ordering::SeqCst);
            let validation = request
                .validate(
                    &record
                        .lock()
                        .map_err(|_| anyhow::anyhow!("session registry poisoned"))?
                        .session_id,
                    last,
                )
                .and_then(|_| {
                    if valid_transition(previous, request.phase) {
                        Ok(())
                    } else {
                        Err(format!(
                            "invalid phase transition {:?} -> {:?}",
                            previous, request.phase
                        ))
                    }
                });
            let mut error = validation.err();
            if error.is_none() {
                error = verify_process_tree(chrome_pid, &cgroup)
                    .err()
                    .map(|value| value.to_string());
            }
            let mut action = "observe".to_owned();
            if error.is_none() {
                latest_generation.store(request.generation, Ordering::SeqCst);
                if let Some(task) = pending.take() {
                    task.abort();
                    let _ = task.await;
                }
                if request.phase != Phase::WaitingForModel {
                    let _ = cgroup.thaw();
                    let _ = cgroup.set_cpu_max(None, 100_000);
                }
                permit = match request.phase {
                    Phase::Navigating => Some(scheduler.acquire_navigation().await?),
                    Phase::Acting => {
                        scheduler.stagger_resume().await?;
                        Some(scheduler.acquire_action().await?)
                    }
                    _ => {
                        drop(permit.take());
                        None
                    }
                };
                if request.phase == Phase::WaitingForModel {
                    match select_mechanism(&config, &request) {
                        Ok(mechanism) => {
                            action = mechanism.slug();
                            if let Err(e) = apply_mechanism(
                                &mechanism,
                                request.generation,
                                latest_generation.clone(),
                                cgroup.clone(),
                                &mut pending,
                            ) {
                                error = Some(e.to_string());
                            }
                        }
                        Err(e) => error = Some(e.to_string()),
                    }
                } else {
                    action = "restore-unrestricted".into();
                }
                if error.is_some() {
                    let _ = cgroup.thaw();
                    let _ = cgroup.set_cpu_max(None, 100_000);
                }
                if error.is_none() {
                    previous = Some(request.phase);
                }
            }
            let mut event = RuntimeEvent::new(
                &record
                    .lock()
                    .map_err(|_| anyhow::anyhow!("session registry poisoned"))?
                    .session_id,
                "phase",
            );
            event.phase = Some(request.phase);
            event.action = Some(action);
            event.message = error.clone();
            telemetry.emit(event).await;
            let ack = PhaseAck {
                schema_version: SCHEMA_VERSION,
                session_id: request.session_id.clone(),
                generation: request.generation,
                accepted: error.is_none(),
                active: !cgroup.is_frozen().unwrap_or(false),
                error,
            };
            writer
                .write_all(serde_json::to_string(&ack)?.as_bytes())
                .await?;
            writer.write_all(b"\n").await?;
            if request.phase == Phase::Finished && ack.accepted {
                break;
            }
        }
        if let Some(task) = pending.take() {
            task.abort();
            let _ = task.await;
        }
        drop(permit.take());
        let _ = cgroup.thaw();
        let _ = cgroup.set_cpu_max(None, 100_000);
    }
}

fn verify_process_tree(root_pid: u32, cgroup: &SessionCgroup) -> Result<()> {
    use std::collections::{HashMap, HashSet};
    let owned = cgroup.pids()?.into_iter().collect::<HashSet<_>>();
    if !owned.contains(&root_pid) {
        bail!("Chromium root process {root_pid} escaped its session cgroup");
    }
    let mut parents = HashMap::new();
    for entry in fs::read_dir("/proc")? {
        let path = entry?.path();
        let Some(pid) = path
            .file_name()
            .and_then(|v| v.to_str())
            .and_then(|v| v.parse::<u32>().ok())
        else {
            continue;
        };
        let Ok(stat) = fs::read_to_string(path.join("stat")) else {
            continue;
        };
        let Some(rest) = stat.rsplit_once(')').map(|(_, rest)| rest.trim()) else {
            continue;
        };
        let Some(ppid) = rest
            .split_whitespace()
            .nth(1)
            .and_then(|v| v.parse::<u32>().ok())
        else {
            continue;
        };
        parents.insert(pid, ppid);
    }
    for &pid in parents.keys() {
        let mut cursor = pid;
        let mut is_descendant = false;
        for _ in 0..64 {
            if cursor == root_pid {
                is_descendant = true;
                break;
            }
            let Some(parent) = parents.get(&cursor) else {
                break;
            };
            if *parent == 0 || *parent == cursor {
                break;
            }
            cursor = *parent;
        }
        if is_descendant && !owned.contains(&pid) {
            bail!("Chromium descendant {pid} escaped session cgroup");
        }
    }
    Ok(())
}

fn select_mechanism(config: &RuntimeConfig, request: &PhaseRequest) -> Result<Mechanism> {
    let expected = request.expected_wait_ms.unwrap_or(0);
    Ok(match config.policy.mode {
        PolicyMode::Observe => Mechanism::Baseline,
        PolicyMode::Safe => {
            if expected >= 2_000 {
                Mechanism::CpuQuota {
                    quota_us: 50_000,
                    period_us: 100_000,
                }
            } else {
                Mechanism::Baseline
            }
        }
        PolicyMode::Adaptive => {
            if expected < 2_000 || request.critical_live_connection {
                Mechanism::Baseline
            } else if config.policy.freeze_compatibility == FreezeCompatibility::Confirmed
                && compatibility_allows_freeze(config, request)?
            {
                Mechanism::CgroupFreeze { delay_ms: 500 }
            } else {
                Mechanism::CpuQuota {
                    quota_us: 25_000,
                    period_us: 100_000,
                }
            }
        }
        PolicyMode::Explicit => config
            .policy
            .explicit
            .as_ref()
            .context("missing explicit policy")?
            .mechanism()?,
    })
}

fn compatibility_allows_freeze(config: &RuntimeConfig, request: &PhaseRequest) -> Result<bool> {
    let path = config
        .policy
        .compatibility_file
        .as_ref()
        .context("missing compatibility file")?;
    let raw = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let file: CompatibilityFile =
        toml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))?;
    let framework = request
        .framework
        .as_deref()
        .context("freeze compatibility requires framework metadata")?;
    let workload = request
        .workload
        .as_deref()
        .context("freeze compatibility requires workload metadata")?;
    let browser = request
        .browser_version
        .as_deref()
        .context("freeze compatibility requires browser_version metadata")?;
    Ok(file.records.iter().any(|record| {
        record.compatible
            && record.mechanism == "cgroup-freeze"
            && record.framework == framework
            && record.workload == workload
            && record.browser_version == browser
    }))
}

fn apply_mechanism(
    mechanism: &Mechanism,
    generation: u64,
    latest: Arc<AtomicU64>,
    cgroup: Arc<SessionCgroup>,
    pending: &mut Option<tokio::task::JoinHandle<()>>,
) -> Result<()> {
    match mechanism {
        Mechanism::Baseline | Mechanism::ChromeLifecycleFreeze => Ok(()),
        Mechanism::CpuQuota {
            quota_us,
            period_us,
        } => cgroup.set_cpu_max(Some(*quota_us), *period_us),
        Mechanism::CgroupFreeze { delay_ms } => {
            let delay = *delay_ms;
            *pending = Some(tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(delay)).await;
                if latest.load(Ordering::SeqCst) == generation {
                    let _ = cgroup.freeze();
                }
            }));
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn adaptive_never_freezes_unknown_or_live_sessions() {
        let config = RuntimeConfig::default();
        let request = PhaseRequest {
            schema_version: 1,
            session_id: "s".into(),
            generation: 1,
            phase: Phase::WaitingForModel,
            expected_wait_ms: Some(5_000),
            page_cdp_session_id: None,
            framework: None,
            workload: None,
            browser_version: None,
            critical_live_connection: false,
        };
        assert!(matches!(
            select_mechanism(&config, &request).unwrap(),
            Mechanism::CpuQuota { .. }
        ));
        let mut live = request;
        live.critical_live_connection = true;
        assert_eq!(
            select_mechanism(&config, &live).unwrap(),
            Mechanism::Baseline
        );
    }
}
