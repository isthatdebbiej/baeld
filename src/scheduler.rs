use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use std::os::fd::AsRawFd;

pub struct Permit {
    path: PathBuf,
}

impl Drop for Permit {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

pub struct AdmissionController {
    root: PathBuf,
    navigation: usize,
    actions: usize,
    stagger: Duration,
    cpu_pressure: f64,
    memory_pressure: f64,
}

impl AdmissionController {
    pub fn new(
        root: &Path,
        navigation: usize,
        actions: usize,
        stagger: Duration,
        cpu_pressure: f64,
        memory_pressure: f64,
    ) -> Result<Self> {
        fs::create_dir_all(root)?;
        Ok(Self {
            root: root.to_owned(),
            navigation,
            actions,
            stagger,
            cpu_pressure,
            memory_pressure,
        })
    }

    pub async fn acquire_navigation(&self) -> Result<Permit> {
        self.acquire("navigation", self.navigation).await
    }
    pub async fn acquire_action(&self) -> Result<Permit> {
        self.acquire("action", self.actions).await
    }

    async fn acquire(&self, class: &str, limit: usize) -> Result<Permit> {
        loop {
            self.wait_for_pressure().await;
            self.remove_stale(class)?;
            for slot in 0..limit {
                let path = self.root.join(format!("{class}-{slot}.permit"));
                match OpenOptions::new().create_new(true).write(true).open(&path) {
                    Ok(mut file) => {
                        writeln!(file, "{}", std::process::id())?;
                        return Ok(Permit { path });
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                    Err(error) => return Err(error.into()),
                }
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }

    async fn wait_for_pressure(&self) {
        loop {
            let cpu = pressure_avg10("/proc/pressure/cpu").unwrap_or(0.0);
            let memory = pressure_avg10("/proc/pressure/memory").unwrap_or(0.0);
            if cpu < self.cpu_pressure && memory < self.memory_pressure {
                return;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    fn remove_stale(&self, class: &str) -> Result<()> {
        for entry in fs::read_dir(&self.root)? {
            let path = entry?.path();
            if !path
                .file_name()
                .and_then(|v| v.to_str())
                .is_some_and(|v| v.starts_with(class) && v.ends_with(".permit"))
            {
                continue;
            }
            let pid = fs::read_to_string(&path)
                .ok()
                .and_then(|v| v.trim().parse::<u32>().ok());
            if pid.is_none_or(|pid| !Path::new(&format!("/proc/{pid}")).exists()) {
                let _ = fs::remove_file(path);
            }
        }
        Ok(())
    }

    pub async fn stagger_resume(&self) -> Result<()> {
        let path = self.root.join("resume.lock");
        let mut file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(path)?;
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) } != 0 {
            return Err(std::io::Error::last_os_error()).context("locking global resume gate");
        }
        let mut raw = String::new();
        file.read_to_string(&mut raw)?;
        let previous = raw.trim().parse::<u128>().unwrap_or(0);
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
        let target = previous.saturating_add(self.stagger.as_millis());
        if target > now {
            tokio::time::sleep(Duration::from_millis((target - now) as u64)).await;
        }
        let updated = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
        file.set_len(0)?;
        write!(file, "{updated}")?;
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_UN) } != 0 {
            return Err(std::io::Error::last_os_error()).context("unlocking global resume gate");
        }
        Ok(())
    }
}

fn pressure_avg10(path: &str) -> Option<f64> {
    let raw = fs::read_to_string(path).ok()?;
    raw.lines()
        .find(|line| line.starts_with("some "))?
        .split_whitespace()
        .find_map(|field| {
            field
                .strip_prefix("avg10=")
                .and_then(|value| value.parse().ok())
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_pressure_file_fails_open() {
        assert_eq!(pressure_avg10("/definitely/missing/baeld-pressure"), None);
    }
}
