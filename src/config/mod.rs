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
}

impl Default for AppConfig {
    fn default() -> Self {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("singboxer");

        let subscriptions_file = config_dir.join("subscriptions.json");
        let singbox_config_dir = config_dir.join("singbox");

        Self {
            config_dir,
            subscriptions_file,
            singbox_config_dir,
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
}

/// Generate sing-box configuration from proxy servers
pub fn generate_singbox_config(
    proxies: &[ProxyServer],
    selected_proxy: Option<&str>,
) -> Result<SingBoxConfig> {
    let mut config = SingBoxConfig::default();

    // Add TUN inbound (using modern 'address' field instead of deprecated inet4_address/inet6_address)
    config.inbounds = vec![
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
    ];

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

    // Add direct and block first
    outbounds.push(serde_json::json!({
        "type": "direct",
        "tag": "direct"
    }));

    outbounds.push(serde_json::json!({
        "type": "block",
        "tag": "block"
    }));

    outbounds.push(serde_json::json!({
        "type": "dns",
        "tag": "dns-out"
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

    // Add DNS configuration
    config.dns = Some(serde_json::json!({
        "servers": [
            {
                "tag": "local",
                "address": "https://223.5.5.5/dns-query",
                "strategy": "prefer_ipv4"
            },
            {
                "tag": "remote",
                "address": "https://1.1.1.1/dns-query",
                "strategy": "prefer_ipv4",
                "address_resolver": "local"
            }
        ],
        "final": "local",
        "strategy": "prefer_ipv4",
        "disable_cache": false
    }));

    // Add routing rules with sniff configuration
    config.route = Some(serde_json::json!({
        "rules": [
            // DNS queries
            {
                "protocol": "dns",
                "outbound": "dns-out"
            },
            // Private networks - direct
            {
                "geoip": "private",
                "outbound": "direct"
            },
            // China IPs - direct
            {
                "geoip": "cn",
                "outbound": "direct"
            },
            // China domains - direct
            {
                "geosite": "cn",
                "outbound": "direct"
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
