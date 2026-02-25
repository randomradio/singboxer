// Configuration management for singboxer

use crate::models::*;
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

/// App configuration
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub config_dir: PathBuf,
    pub subscriptions_file: PathBuf,
    pub singbox_config_dir: PathBuf,
    pub proxy_cache_file: PathBuf,
}

impl Default for AppConfig {
    fn default() -> Self {
        // Use ~/.config/singboxer for subscriptions
        // Use ~/.config/sing-box for sing-box configs
        let base_config = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from(".config"));

        let config_dir = base_config.join("singboxer");
        let subscriptions_file = config_dir.join("subscriptions.json");
        let singbox_config_dir = base_config.join("sing-box");
        let proxy_cache_file = config_dir.join("proxy_cache.json");

        Self {
            config_dir,
            subscriptions_file,
            singbox_config_dir,
            proxy_cache_file,
        }
    }
}

impl AppConfig {
    /// Ensure config directories exist
    pub fn init(&self) -> Result<()> {
        fs::create_dir_all(&self.config_dir)
            .context("Failed to create config directory")?;
        fs::create_dir_all(&self.singbox_config_dir)
            .context("Failed to create singbox config directory")?;
        Ok(())
    }

    /// Load subscriptions from file
    pub fn load_subscriptions(&self) -> Result<Vec<Subscription>> {
        if !self.subscriptions_file.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&self.subscriptions_file)
            .context("Failed to read subscriptions file")?;

        let subs: Vec<Subscription> = serde_json::from_str(&content)
            .context("Failed to parse subscriptions")?;

        Ok(subs)
    }

    /// Save subscriptions to file
    pub fn save_subscriptions(&self, subs: &[Subscription]) -> Result<()> {
        let content = serde_json::to_string_pretty(subs)
            .context("Failed to serialize subscriptions")?;

        fs::write(&self.subscriptions_file, content)
            .context("Failed to write subscriptions file")?;

        Ok(())
    }

    /// Save proxy list to cache file
    pub fn save_proxy_cache(&self, proxies: &[ProxyServer]) -> Result<()> {
        let content = serde_json::to_string_pretty(proxies)
            .context("Failed to serialize proxy cache")?;

        fs::write(&self.proxy_cache_file, content)
            .context("Failed to write proxy cache file")?;

        Ok(())
    }

    /// Load proxy list from cache file
    pub fn load_proxy_cache(&self) -> Result<Vec<ProxyServer>> {
        if !self.proxy_cache_file.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&self.proxy_cache_file)
            .context("Failed to read proxy cache file")?;

        let proxies: Vec<ProxyServer> = serde_json::from_str(&content)
            .context("Failed to parse proxy cache")?;

        Ok(proxies)
    }
}

/// Generate sing-box configuration from proxy servers
///
/// # Arguments
/// * `proxies` - List of proxy servers
/// * `selected_proxy` - Name of the currently selected proxy
/// * `no_tun` - If true, exclude TUN inbound (useful for systems without TUN permissions)
pub fn generate_singbox_config(
    proxies: &[ProxyServer],
    selected_proxy: Option<&str>,
    no_tun: bool,
) -> Result<SingBoxConfig> {
    let mut config = SingBoxConfig::default();

    // Add TUN inbound (using modern 'address' field)
    // Skip if no_tun is true (for systems where TUN requires root)
    if !no_tun {
        config.inbounds.push(
            serde_json::json!({
                "type": "tun",
                "tag": "tun-in",
                "address": [
                    "172.19.0.1/30",
                    "fdfe:dcba:9876::1/126"
                ],
                "auto_route": true,
                "strict_route": false,
                "mtu": 9000
            })
        );
    }

    // Add SOCKS inbound for local use
    config.inbounds.push(
        serde_json::json!({
            "type": "socks",
            "tag": "socks-in",
            "listen": "127.0.0.1",
            "listen_port": 7890
        })
    );

    // Add HTTP inbound
    config.inbounds.push(
        serde_json::json!({
            "type": "http",
            "tag": "http-in",
            "listen": "127.0.0.1",
            "listen_port": 7891
        })
    );

    // Convert proxies to outbounds
    let proxy_tags: Vec<String> = proxies
        .iter()
        .map(|p| sanitize_tag(&p.name))
        .collect();

    let mut outbounds = Vec::new();

    // Add direct outbound
    outbounds.push(serde_json::json!({
        "type": "direct",
        "tag": "direct"
    }));

    // Add selector for manual selection
    if !proxy_tags.is_empty() {
        outbounds.push(serde_json::json!({
            "type": "selector",
            "tag": "proxy",
            "outbounds": proxy_tags,
            "default": selected_proxy.map(|s| sanitize_tag(s)).unwrap_or_else(|| proxy_tags[0].clone())
        }));
    }

    // Add urltest for auto selection
    if !proxy_tags.is_empty() {
        outbounds.push(serde_json::json!({
            "type": "urltest",
            "tag": "auto",
            "outbounds": proxy_tags,
            "url": "http://www.gstatic.com/generate_204",
            "interval": "5m",
            "tolerance": 50
        }));
    }

    // Add all proxy outbounds
    for proxy in proxies {
        outbounds.push(crate::parser::proxy_to_outbound(proxy));
    }

    config.outbounds = outbounds;

    // DNS configuration - sing-box 1.12+ format
    // New format uses "type" and "server" instead of "address"
    config.dns = Some(serde_json::json!({
        "servers": [
            {
                "tag": "local-dns",
                "type": "udp",
                "server": "223.5.5.5"
            },
            {
                "tag": "remote-dns",
                "type": "https",
                "server": "1.1.1.1"
            }
        ],
        "final": "local-dns",
        "strategy": "prefer_ipv4",
        "disable_cache": false,
        "disable_expire": false
    }));

    // Route configuration - sing-box 1.12+ requires domain_resolver
    config.route = Some(serde_json::json!({
        "default_domain_resolver": "local-dns",
        "rules": [
            // Private networks - direct
            {
                "ip_is_private": true,
                "action": "direct"
            },
            // DNS queries - use route action
            {
                "protocol": "dns",
                "action": "hijack-dns"
            }
        ],
        "final": "proxy",
        "auto_detect_interface": true
    }));

    Ok(config)
}

/// Save sing-box config to file
pub fn save_singbox_config(config: &SingBoxConfig, path: &PathBuf) -> Result<()> {
    let content = serde_json::to_string_pretty(config)
        .context("Failed to serialize config")?;

    fs::write(path, content)
        .context("Failed to write config file")?;

    Ok(())
}

fn sanitize_tag(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}
