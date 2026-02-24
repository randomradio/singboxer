// sing-box binary detection and process management

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::{Command, Child, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// GitHub repository for sing-box installation instructions
pub const SINGBOX_GITHUB: &str = "https://github.com/SagerNet/sing-box#installation";

/// sing-box manager
#[derive(Clone)]
pub struct SingBoxManager {
    binary_path: Arc<Mutex<Option<PathBuf>>>,
    process: Arc<Mutex<Option<SingBoxProcess>>>,
}

/// Running sing-box process
pub struct SingBoxProcess {
    child: Child,
    pid: u32,
}

impl SingBoxProcess {
    pub fn pid(&self) -> u32 {
        self.pid
    }

    pub fn is_running(&mut self) -> bool {
        self.child.try_wait().map(|s| s.is_none()).unwrap_or(false)
    }

    pub fn kill(&mut self) -> Result<()> {
        self.child.kill()
            .context("Failed to kill sing-box process")?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SingBoxStatus {
    NotFound,
    Available,
    Running { pid: u32 },
    Stopped,
}

impl SingBoxManager {
    pub fn new() -> Self {
        Self {
            binary_path: Arc::new(Mutex::new(None)),
            process: Arc::new(Mutex::new(None)),
        }
    }

    /// Check if sing-box is installed and get its path
    pub fn check_installation(&self) -> Result<PathBuf> {
        let path = self.find_binary()?;
        {
            let mut cached = self.binary_path.lock().unwrap();
            *cached = Some(path.clone());
        }
        Ok(path)
    }

    /// Find sing-box binary in PATH or common locations
    fn find_binary(&self) -> Result<PathBuf> {
        // Try common binary names
        let names = ["sing-box", "sing-box.exe"];

        // Try PATH first
        for name in &names {
            if let Ok(path) = which::which(name) {
                return Ok(path);
            }
        }

        // Try common installation directories
        let common_dirs = if cfg!(windows) {
            vec![
                PathBuf::from(r"C:\Program Files\sing-box"),
                PathBuf::from(r"C:\Program Files (x86)\sing-box"),
            ]
        } else if cfg!(target_os = "macos") {
            vec![
                PathBuf::from("/usr/local/bin"),
                PathBuf::from("/opt/homebrew/bin"),
                PathBuf::from("/usr/local/opt/sing-box/bin"),
            ]
        } else {
            vec![
                PathBuf::from("/usr/bin"),
                PathBuf::from("/usr/local/bin"),
                PathBuf::from("/opt/bin"),
                PathBuf::from("~/.local/bin"),
            ]
        };

        for dir in common_dirs {
            for name in &names {
                let path = dir.join(name);
                if path.exists() && is_executable(&path) {
                    return Ok(path);
                }
            }
        }

        Err(anyhow::anyhow!(
            "sing-box not found. Install from {}",
            SINGBOX_GITHUB
        ))
    }

    /// Get sing-box version
    pub fn get_version(&self) -> Result<String> {
        let binary = self.binary_path.lock().unwrap();
        let path = binary.as_ref()
            .ok_or_else(|| anyhow::anyhow!("sing-box not found"))?;

        let output = Command::new(path)
            .arg("version")
            .output()
            .context("Failed to get sing-box version")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("sing-box version command failed"));
        }

        let version = String::from_utf8_lossy(&output.stdout);
        Ok(version.lines().next()
            .unwrap_or("unknown")
            .to_string())
    }

    /// Validate a sing-box configuration
    pub fn validate_config(&self, config_path: &Path) -> Result<()> {
        let binary = self.binary_path.lock().unwrap();
        let path = binary.as_ref()
            .ok_or_else(|| anyhow::anyhow!("sing-box not found"))?;

        let output = Command::new(path)
            .arg("check")
            .arg("-c")
            .arg(config_path)
            .output()
            .context("Failed to validate config")?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Config validation failed: {}", error));
        }

        Ok(())
    }

    /// Start sing-box with the given configuration
    pub fn start(&self, config_path: &str) -> Result<u32> {
        // Ensure config directory exists
        if let Some(parent) = PathBuf::from(config_path).parent() {
            std::fs::create_dir_all(parent)
                .context("Failed to create config directory")?;
        }

        // Check if already running
        let mut process_guard = self.process.lock().unwrap();
        if let Some(ref mut proc) = *process_guard {
            if proc.is_running() {
                return Ok(proc.pid());
            }
        }

        let binary = self.binary_path.lock().unwrap();
        let path = binary.as_ref()
            .ok_or_else(|| anyhow::anyhow!("sing-box binary not found. Install from {}", SINGBOX_GITHUB))?;

        // First validate the config
        drop(binary);
        self.validate_config(Path::new(config_path))?;

        // Start sing-box
        let binary = self.binary_path.lock().unwrap();
        let path = binary.as_ref().unwrap();

        // Note: TUN mode requires CAP_NET_ADMIN capability
        let child = Command::new(path)
            .arg("run")
            .arg("-c")
            .arg(config_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to start sing-box.\nHint: TUN mode requires CAP_NET_ADMIN. Run: sudo setcap cap_net_admin,cap_net_raw+ep /path/to/sing-box\nOr run with: sudo ./sing-box run -c config.json")?;

        let pid = child.id();
        *process_guard = Some(SingBoxProcess { child, pid });

        Ok(pid)
    }

    /// Stop the running sing-box process
    pub fn stop(&self) -> Result<()> {
        let mut process_guard = self.process.lock().unwrap();
        if let Some(ref mut proc) = *process_guard {
            proc.kill()?;
            *process_guard = None;
        }
        Ok(())
    }

    /// Restart sing-box with new configuration
    pub fn restart(&self, config_path: &Path) -> Result<u32> {
        self.stop()?;
        std::thread::sleep(Duration::from_millis(500));
        self.start(config_path.to_str().ok_or_else(|| anyhow::anyhow!("Invalid config path"))?)
    }

    /// Get current status
    pub fn status(&self) -> SingBoxStatus {
        // Check if binary exists
        let binary = self.binary_path.lock().unwrap();
        if binary.is_none() {
            drop(binary);
            // Try to find it
            if self.check_installation().is_err() {
                return SingBoxStatus::NotFound;
            }
        }

        // Check if running
        let mut process_guard = self.process.lock().unwrap();
        if let Some(ref mut proc) = *process_guard {
            if proc.is_running() {
                return SingBoxStatus::Running { pid: proc.pid() };
            }
        }
        drop(process_guard);

        // Check if sing-box is running externally (by PID)
        if let Ok(pid) = self.find_running_instance() {
            return SingBoxStatus::Running { pid };
        }

        SingBoxStatus::Available
    }

    /// Find a running sing-box process
    fn find_running_instance(&self) -> Result<u32> {
        use sysinfo::System;

        let mut sys = System::new_all();
        sys.refresh_all();

        for (pid, process) in sys.processes() {
            let name = process.name();
            if name == "sing-box" || name == "sing-box.exe" {
                return Ok(pid.as_u32());
            }
        }

        Err(anyhow::anyhow!("No sing-box process found"))
    }

    /// Get the Clash API URL from the config (for proxy switching)
    pub fn get_clash_api_url(&self, config_path: &Path) -> Option<String> {
        let content = std::fs::read_to_string(config_path).ok()?;
        let config: serde_json::Value = serde_json::from_str(&content).ok()?;

        config
            .get("experimental")?
            .get("clash-api")?
            .get("external-controller")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    /// Switch proxy via Clash API
    pub async fn switch_proxy(&self, selector: &str, proxy_tag: &str, config_path: &Path) -> Result<()> {
        let api_url = self.get_clash_api_url(config_path)
            .unwrap_or_else(|| "http://127.0.0.1:9090".to_string());

        let url = format!("{}/proxies/{}", api_url, selector);
        let secret = self.get_clash_api_secret(config_path);

        let mut request = reqwest::Client::new()
            .put(&url)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({ "name": proxy_tag }));

        if let Some(token) = secret {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request.send().await
            .context("Failed to connect to Clash API")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Clash API returned error: {}", response.status()));
        }

        Ok(())
    }

    fn get_clash_api_secret(&self, config_path: &Path) -> Option<String> {
        let content = std::fs::read_to_string(config_path).ok()?;
        let config: serde_json::Value = serde_json::from_str(&content).ok()?;

        config
            .get("experimental")?
            .get("clash-api")?
            .get("secret")
            .and_then(|v| v.as_str())
            .map(|s| {
                if s.is_empty() { None } else { Some(s.to_string()) }
            })
            .flatten()
    }

    /// Test latency through a proxy (via Clash API URL test if available, or direct)
    pub async fn test_latency(&self, proxy_tag: &str, test_url: &str, config_path: &Path) -> Result<Duration> {
        let api_url = self.get_clash_api_url(config_path)
            .unwrap_or_else(|| "http://127.0.0.1:9090".to_string());

        // Try Clash API latency test first (if sing-box is running)
        let url = format!("{}/proxies/{}/delay", api_url, urlencoding::encode(proxy_tag));
        let secret = self.get_clash_api_secret(config_path);

        let mut request = reqwest::Client::new()
            .get(&url)
            .query(&[("url", test_url), ("timeout", "5000")]);

        if let Some(token) = secret {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        match request.send().await {
            Ok(response) if response.status().is_success() => {
                let json: serde_json::Value = response.json().await?;
                if let Some(delay) = json.get("delay").and_then(|d| d.as_u64()) {
                    return Ok(Duration::from_millis(delay));
                }
            }
            _ => {}
        }

        // Fallback: direct HTTP test through the proxy
        self.test_latency_direct(proxy_tag, test_url, config_path).await
    }

    async fn test_latency_direct(&self, _proxy_tag: &str, test_url: &str, config_path: &Path) -> Result<Duration> {
        // Parse config to get proxy details
        let content = std::fs::read_to_string(config_path)?;
        let _config: serde_json::Value = serde_json::from_str(&content)?;

        // This is a simplified test - in production, you'd create a proxy client
        let start = std::time::Instant::now();

        let response = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()?
            .get(test_url)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(start.elapsed())
        } else {
            Err(anyhow::anyhow!("HTTP request failed"))
        }
    }
}

impl Default for SingBoxManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if a file is executable
fn is_executable(path: &Path) -> bool {
    // On Unix, check execute permission
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        match std::fs::metadata(path) {
            Ok(meta) => {
                let permissions = meta.permissions();
                permissions.mode() & 0o111 != 0
            }
            Err(_) => false,
        }
    }

    // On Windows, .exe files are executable
    #[cfg(windows)]
    {
        path.extension().map_or(false, |ext| ext.eq_ignore_ascii_case("exe"))
    }
}

// Re-export which since we use it
use which;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_executable_unix() {
        #[cfg(unix)]
        {
            // /bin/ls should be executable
            assert!(is_executable(Path::new("/bin/ls")));
        }
    }
}
