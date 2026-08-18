use std::env;
use std::fs;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use anyhow::{bail, Result};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct Check {
    name: &'static str,
    ok: bool,
    detail: String,
}

pub async fn run(json: bool) -> Result<()> {
    let checks = vec![
        Check {
            name: "linux",
            ok: cfg!(target_os = "linux"),
            detail: env::consts::OS.into(),
        },
        file_check("cgroup-v2", "/sys/fs/cgroup/cgroup.controllers"),
        file_check("cpu-pressure", "/proc/pressure/cpu"),
        file_check("memory-pressure", "/proc/pressure/memory"),
        file_check("io-pressure", "/proc/pressure/io"),
        command_check("systemd-run", &["--version"]),
        command_check("node", &["--version"]),
        named_command_check(
            "playwright",
            "node",
            &[
                "-e",
                "import('playwright').then(p => console.log(p.chromium.executablePath()))",
            ],
        ),
        command_check("taskset", &["--version"]),
        chromium_check(),
        sandbox_check(),
        active_cgroup_check(),
    ];

    if json {
        println!("{}", serde_json::to_string_pretty(&checks)?);
    } else {
        println!("Baeld host diagnostics\n");
        for check in &checks {
            println!(
                "{:<22} {:<5} {}",
                check.name,
                if check.ok { "PASS" } else { "FAIL" },
                check.detail
            );
        }
    }

    if checks.iter().any(|check| !check.ok) {
        bail!("host is not ready for publishable Baeld experiments");
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn active_cgroup_check() -> Check {
    use crate::cgroup::{current_cgroup_path, prepare_delegated_parent, SessionCgroup};
    let outcome = (|| -> Result<()> {
        let parent = prepare_delegated_parent(&current_cgroup_path()?)?;
        let group = SessionCgroup::create(&parent, "doctor")?;
        let script = format!("{} sleep 5", group.join_command_prefix());
        let mut child = Command::new("/bin/sh").args(["-c", &script]).spawn()?;
        std::thread::sleep(std::time::Duration::from_millis(100));
        group.freeze()?;
        group.thaw()?;
        let _ = child.kill();
        let _ = child.wait();
        group.remove()?;
        Ok(())
    })();
    Check {
        name: "cgroup-active-test",
        ok: outcome.is_ok(),
        detail: outcome
            .map(|_| "child freeze/thaw succeeded".into())
            .unwrap_or_else(|e| e.to_string()),
    }
}

#[cfg(not(target_os = "linux"))]
fn active_cgroup_check() -> Check {
    Check {
        name: "cgroup-active-test",
        ok: false,
        detail: "requires Linux".into(),
    }
}

fn file_check(name: &'static str, path: &'static str) -> Check {
    Check {
        name,
        ok: Path::new(path).exists(),
        detail: path.into(),
    }
}

fn command_check(name: &'static str, args: &[&str]) -> Check {
    named_command_check(name, name, args)
}

fn named_command_check(name: &'static str, program: &str, args: &[&str]) -> Check {
    match Command::new(program).args(args).output() {
        Ok(output) => Check {
            name,
            ok: output.status.success(),
            detail: String::from_utf8_lossy(&output.stdout)
                .lines()
                .next()
                .unwrap_or("present")
                .to_owned(),
        },
        Err(error) => Check {
            name,
            ok: false,
            detail: error.to_string(),
        },
    }
}

fn chromium_check() -> Check {
    let Some(binary) = find_chromium() else {
        return Check {
            name: "chromium",
            ok: false,
            detail: "no supported Chromium binary found".into(),
        };
    };
    match Command::new(&binary).arg("--version").output() {
        Ok(output) if output.status.success() => Check {
            name: "chromium",
            ok: true,
            detail: String::from_utf8_lossy(&output.stdout).trim().into(),
        },
        Ok(output) => Check {
            name: "chromium",
            ok: false,
            detail: String::from_utf8_lossy(&output.stderr).trim().into(),
        },
        Err(error) => Check {
            name: "chromium",
            ok: false,
            detail: error.to_string(),
        },
    }
}

fn find_chromium() -> Option<String> {
    let candidates = [
        env::var("BAELD_CHROMIUM").ok(),
        Some(".baeld/chromium".into()),
        Some("chromium".into()),
        Some("chromium-browser".into()),
        Some("google-chrome-stable".into()),
        Some("google-chrome".into()),
    ];
    candidates.into_iter().flatten().find(|binary| {
        Command::new(binary)
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success())
    })
}

fn sandbox_check() -> Check {
    let disabled = env::var_os("BAELD_CHROME_ARGS")
        .map(|args| args.to_string_lossy().contains("--no-sandbox"))
        .unwrap_or(false);
    let user_namespace = fs::read_to_string("/proc/sys/kernel/unprivileged_userns_clone")
        .map(|v| v.trim() == "1")
        .unwrap_or(true);
    if disabled {
        return Check {
            name: "chromium-sandbox",
            ok: false,
            detail: "BAELD_CHROME_ARGS contains forbidden --no-sandbox".into(),
        };
    }
    if !user_namespace {
        return Check {
            name: "chromium-sandbox",
            ok: false,
            detail: "unprivileged user namespaces are disabled".into(),
        };
    }
    #[cfg(target_os = "linux")]
    if unsafe { libc::geteuid() } == 0 {
        return Check {
            name: "chromium-sandbox",
            ok: false,
            detail: "refusing to validate Chromium as root".into(),
        };
    }
    let Some(binary) = find_chromium() else {
        return Check {
            name: "chromium-sandbox",
            ok: false,
            detail: "Chromium is unavailable".into(),
        };
    };
    let profile = match tempfile::Builder::new().prefix("baeld-doctor-").tempdir() {
        Ok(profile) => profile,
        Err(error) => {
            return Check {
                name: "chromium-sandbox",
                ok: false,
                detail: error.to_string(),
            };
        }
    };
    let output = chromium_probe(&binary, profile.path());
    match output {
        Ok(output) if output.status.success() => Check {
            name: "chromium-sandbox",
            ok: true,
            detail: "unprivileged launch succeeded without --no-sandbox".into(),
        },
        Ok(output) => Check {
            name: "chromium-sandbox",
            ok: false,
            detail: String::from_utf8_lossy(&output.stderr).trim().into(),
        },
        Err(error) => Check {
            name: "chromium-sandbox",
            ok: false,
            detail: error,
        },
    }
}

fn chromium_probe(binary: &str, profile: &Path) -> std::result::Result<Output, String> {
    let mut child = Command::new(binary)
        .args([
            "--headless=new",
            "--disable-gpu",
            "--disable-background-networking",
            "--disable-component-update",
            "--disable-crash-reporter",
            "--dump-dom",
            &format!("--user-data-dir={}", profile.display()),
            "about:blank",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| error.to_string())?;
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        match child.try_wait().map_err(|error| error.to_string())? {
            Some(_) => return child.wait_with_output().map_err(|error| error.to_string()),
            None if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(50)),
            None => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("Chromium sandbox probe exceeded 15 seconds".into());
            }
        }
    }
}
