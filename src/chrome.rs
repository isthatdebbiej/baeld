use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use tempfile::TempDir;
use tokio::process::{Child, Command};

use crate::cgroup::SessionCgroup;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChromeConfig {
    pub executable: PathBuf,
    #[serde(default = "default_host")]
    pub remote_debugging_host: String,
    #[serde(default)]
    pub extra_args: Vec<String>,
    #[serde(default)]
    pub allow_extensions: bool,
    #[serde(default)]
    pub cpu_affinity: Option<String>,
}

fn default_host() -> String {
    "127.0.0.1".into()
}

pub struct ChromeProcess {
    child: Child,
    _profile: TempDir,
    pub port: u16,
}

impl ChromeProcess {
    pub async fn launch(config: &ChromeConfig, cgroup: &SessionCgroup, port: u16) -> Result<Self> {
        let profile = tempfile::Builder::new().prefix("baeld-chrome-").tempdir()?;
        let args = chrome_arguments(config, profile.path(), port);

        let quoted_exe = shell_quote(&config.executable);
        let quoted_args = args
            .iter()
            .map(|a| shell_quote(Path::new(a)))
            .collect::<Vec<_>>()
            .join(" ");
        let launch = match &config.cpu_affinity {
            Some(cpus) => format!(
                "taskset -c '{}' {} {}",
                cpus.replace('\'', ""),
                quoted_exe,
                quoted_args
            ),
            None => format!("{} {}", quoted_exe, quoted_args),
        };
        let script = format!("{} {}", cgroup.join_command_prefix(), launch);
        let mut command = Command::new("/bin/sh");
        command
            .arg("-c")
            .arg(script)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        let child = command.spawn().context("launching Chromium")?;

        let process = Self {
            child,
            _profile: profile,
            port,
        };
        process.wait_ready(Duration::from_secs(15)).await?;
        Ok(process)
    }

    pub fn websocket_discovery_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    pub async fn wait_ready(&self, timeout: Duration) -> Result<()> {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if tokio::net::TcpStream::connect(("127.0.0.1", self.port))
                .await
                .is_ok()
            {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        bail!(
            "Chromium did not listen on port {} within {timeout:?}",
            self.port
        )
    }

    pub async fn terminate(&mut self) {
        let _ = self.child.kill().await;
        let _ = self.child.wait().await;
    }
}

fn chrome_arguments(config: &ChromeConfig, profile: &Path, port: u16) -> Vec<String> {
    let mut args = vec![
        "--headless=new".to_owned(),
        format!(
            "--remote-debugging-address={}",
            config.remote_debugging_host
        ),
        format!("--remote-debugging-port={port}"),
        format!("--user-data-dir={}", profile.display()),
        "--disable-default-apps".to_owned(),
        "--disable-sync".to_owned(),
        "--no-first-run".to_owned(),
        "--no-default-browser-check".to_owned(),
        "about:blank".to_owned(),
    ];
    if !config.allow_extensions {
        args.insert(5, "--disable-extensions".to_owned());
    }
    args.extend(config.extra_args.clone());
    args
}

fn shell_quote(path: &Path) -> String {
    let value = path.as_os_str().to_string_lossy();
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(allow_extensions: bool) -> ChromeConfig {
        ChromeConfig {
            executable: "/chromium".into(),
            remote_debugging_host: "127.0.0.1".into(),
            extra_args: vec!["--remote-allow-origins=*".into()],
            allow_extensions,
            cpu_affinity: None,
        }
    }

    #[test]
    fn extensions_are_disabled_by_default() {
        let args = chrome_arguments(&config(false), Path::new("/tmp/profile"), 9300);
        assert!(args.iter().any(|arg| arg == "--disable-extensions"));
    }

    #[test]
    fn stagehand_can_explicitly_enable_extensions() {
        let args = chrome_arguments(&config(true), Path::new("/tmp/profile"), 9300);
        assert!(!args.iter().any(|arg| arg == "--disable-extensions"));
        assert!(args.iter().any(|arg| arg == "--remote-allow-origins=*"));
    }
}
