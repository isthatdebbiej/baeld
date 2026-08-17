use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};

use crate::metrics::{self, ResourceSample};

#[derive(Debug)]
pub struct SessionCgroup {
    path: PathBuf,
    removed: bool,
}

impl SessionCgroup {
    pub fn create(parent: &Path, session_id: &str) -> Result<Self> {
        let path = parent.join(format!("session-{session_id}"));
        fs::create_dir(&path).with_context(|| format!("creating cgroup {}", path.display()))?;
        Ok(Self {
            path,
            removed: false,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn join_command_prefix(&self) -> String {
        format!(
            "printf '%s' $$ > '{}' && exec",
            self.path.join("cgroup.procs").display()
        )
    }

    pub fn set_cpu_max(&self, quota_us: Option<u64>, period_us: u64) -> Result<()> {
        let value = quota_us
            .map(|quota| format!("{quota} {period_us}"))
            .unwrap_or_else(|| format!("max {period_us}"));
        self.write("cpu.max", &value)
    }

    pub fn freeze(&self) -> Result<()> {
        self.write("cgroup.freeze", "1")?;
        self.wait_frozen(true, Duration::from_secs(2))
    }

    pub fn thaw(&self) -> Result<()> {
        self.write("cgroup.freeze", "0")?;
        self.wait_frozen(false, Duration::from_secs(2))
    }

    pub fn is_frozen(&self) -> Result<bool> {
        let events = fs::read_to_string(self.path.join("cgroup.events"))?;
        Ok(events.lines().any(|line| line.trim() == "frozen 1"))
    }

    pub fn pids(&self) -> Result<Vec<u32>> {
        Ok(fs::read_to_string(self.path.join("cgroup.procs"))?
            .lines()
            .filter_map(|line| line.trim().parse().ok())
            .collect())
    }

    pub fn sample(&self) -> Result<ResourceSample> {
        metrics::sample(&self.path)
    }

    pub fn kill_all(&self) -> Result<()> {
        let kill_path = self.path.join("cgroup.kill");
        if kill_path.exists() {
            fs::write(&kill_path, "1")
                .with_context(|| format!("writing {}", kill_path.display()))?;
            return Ok(());
        }
        for pid in self.pids()? {
            unsafe {
                libc::kill(pid as libc::pid_t, libc::SIGKILL);
            }
        }
        Ok(())
    }

    pub fn remove(mut self) -> Result<()> {
        self.thaw().ok();
        self.kill_all().ok();
        let deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < deadline && !self.pids().unwrap_or_default().is_empty() {
            std::thread::sleep(Duration::from_millis(25));
        }
        fs::remove_dir(&self.path)
            .with_context(|| format!("removing cgroup {}", self.path.display()))?;
        self.removed = true;
        Ok(())
    }

    fn write(&self, file: &str, value: &str) -> Result<()> {
        let path = self.path.join(file);
        fs::write(&path, value).with_context(|| format!("writing {value:?} to {}", path.display()))
    }

    fn wait_frozen(&self, wanted: bool, timeout: Duration) -> Result<()> {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if self.is_frozen()? == wanted {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        bail!(
            "cgroup {} did not reach frozen={wanted}",
            self.path.display()
        )
    }
}

impl Drop for SessionCgroup {
    fn drop(&mut self) {
        if self.removed {
            return;
        }
        let _ = self.thaw();
        let _ = self.kill_all();
        let deadline = Instant::now() + Duration::from_secs(1);
        while Instant::now() < deadline && !self.pids().unwrap_or_default().is_empty() {
            std::thread::sleep(Duration::from_millis(10));
        }
        let _ = fs::remove_dir(&self.path);
    }
}

pub fn current_cgroup_path() -> Result<PathBuf> {
    let cgroup = fs::read_to_string("/proc/self/cgroup")?;
    let entry = cgroup
        .lines()
        .find_map(|line| line.strip_prefix("0::"))
        .context("unified cgroup v2 entry not found")?;
    Ok(Path::new("/sys/fs/cgroup").join(entry.trim_start_matches('/')))
}

pub fn prepare_delegated_parent(parent: &Path) -> Result<PathBuf> {
    let controller = parent.join(format!("controller-{}", std::process::id()));
    fs::create_dir(&controller)
        .with_context(|| format!("creating controller cgroup {}", controller.display()))?;
    fs::write(
        controller.join("cgroup.procs"),
        std::process::id().to_string(),
    )
    .context("moving Baeld into controller subgroup")?;

    let available = fs::read_to_string(parent.join("cgroup.controllers"))?;
    let requested = ["cpu", "memory", "io", "pids"]
        .into_iter()
        .filter(|name| available.split_whitespace().any(|value| value == *name))
        .map(|name| format!("+{name}"))
        .collect::<Vec<_>>();
    if requested.len() != 4 {
        bail!("delegated scope does not expose cpu, memory, io, and pids controllers");
    }
    fs::write(parent.join("cgroup.subtree_control"), requested.join(" "))
        .context("enabling delegated cgroup controllers")?;
    Ok(parent.to_owned())
}
