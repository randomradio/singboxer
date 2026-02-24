// Core data models for singboxer

use serde::{Deserialize, Serialize};

/// Subscription source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub name: String,
    pub url: String,
    #[serde(rename = "type")]
    pub sub_type: SubscriptionType,
    pub enabled: bool,
}

impl Subscription {
    pub fn new(name: String, url: String, sub_type: SubscriptionType) -> Self {
        Self {
            name,
            url,
            sub_type,
            enabled: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SubscriptionType {
    Clash,
    Shadowsocks,
    V2Ray,
    Singbox,
    Auto,
}

/// A proxy server parsed from subscription
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyServer {
    pub name: String,
    #[serde(rename = "type")]
    pub proxy_type: ProxyType,
    pub server: String,
    pub port: u16,
    pub country: Option<String>,
    pub latency_ms: Option<u64>,
    pub config: ProxyConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ProxyType {
    Shadowsocks,
    Vmess,
    Vless,
    Trojan,
    Hysteria2,
    Tuic,
    Socks,
    Http,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[serde(tag = "type")]
pub enum ProxyConfig {
    Shadowsocks(ShadowsocksConfig),
    Vmess(VmessConfig),
    Vless(VlessConfig),
    Trojan(TrojanConfig),
    Hysteria2(Hysteria2Config),
    Socks(SocksConfig),
    Http(HttpConfig),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowsocksConfig {
    pub method: String,
    pub password: String,
    pub plugin: Option<String>,
    #[serde(rename = "plugin-opts")]
    pub plugin_opts: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmessConfig {
    pub uuid: String,
    #[serde(rename = "alter_id")]
    pub alter_id: u32,
    pub security: String,
    #[serde(rename = "network")]
    pub transport: String,
    #[serde(rename = "tls")]
    pub tls_enabled: Option<bool>,
    pub servername: Option<String>,
    pub path: Option<String>,
    pub host: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VlessConfig {
    pub uuid: String,
    pub flow: Option<String>,
    #[serde(rename = "network")]
    pub transport: String,
    #[serde(rename = "tls")]
    pub tls_enabled: Option<bool>,
    pub servername: Option<String>,
    pub reality: Option<RealityConfig>,
    pub path: Option<String>,
    pub host: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealityConfig {
    pub enabled: bool,
    #[serde(rename = "public-key")]
    pub public_key: String,
    #[serde(rename = "short-id")]
    pub short_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrojanConfig {
    pub password: String,
    #[serde(rename = "network")]
    pub transport: Option<String>,
    #[serde(rename = "tls")]
    pub tls: Option<TlsConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hysteria2Config {
    pub password: String,
    #[serde(rename = "obfs")]
    pub obfs: Option<String>,
    #[serde(rename = "obfs-password")]
    pub obfs_password: Option<String>,
    #[serde(rename = "sni")]
    pub server_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocksConfig {
    pub username: Option<String>,
    pub password: Option<String>,
    pub udp: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpConfig {
    pub username: Option<String>,
    pub password: Option<String>,
    #[serde(rename = "tls")]
    pub tls: Option<TlsConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    pub enabled: bool,
    #[serde(rename = "server-name")]
    pub server_name: Option<String>,
    pub insecure: Option<bool>,
    pub alpn: Option<Vec<String>>,
}

/// Sing-box outbound configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SingBoxOutbound {
    #[serde(rename = "type")]
    pub outbound_type: String,
    pub tag: String,
    #[serde(flatten)]
    pub config: serde_json::Value,
}

/// Full sing-box configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SingBoxConfig {
    pub log: Option<LogConfig>,
    pub experimental: Option<ExperimentalConfig>,
    pub dns: Option<serde_json::Value>,
    pub inbounds: Vec<serde_json::Value>,
    pub outbounds: Vec<serde_json::Value>,
    pub route: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    pub level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentalConfig {
    #[serde(rename = "clash-api")]
    pub clash_api: Option<ClashApiConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClashApiConfig {
    #[serde(rename = "external-controller")]
    pub external_controller: String,
    pub secret: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsConfig {
    pub servers: Vec<DnsServer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsServer {
    pub tag: String,
    #[serde(rename = "address")]
    pub address: String,
    #[serde(rename = "address-resolver")]
    pub address_resolver: Option<String>,
    #[serde(rename = "strategy")]
    pub strategy: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboundConfig {
    #[serde(rename = "type")]
    pub inbound_type: String,
    pub tag: String,
    pub listen: Option<String>,
    #[serde(rename = "listen-port")]
    pub listen_port: Option<u16>,
    #[serde(rename = "sniff")]
    pub sniff: Option<bool>,
    #[serde(rename = "sniff-override destination")]
    pub sniff_override_destination: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteConfig {
    pub rules: Vec<serde_json::Value>,
    pub final_outbound: Option<String>,
    #[serde(rename = "auto-detect-interface")]
    pub auto_detect_interface: Option<bool>,
}

impl Default for SingBoxConfig {
    fn default() -> Self {
        Self {
            log: Some(LogConfig {
                level: "info".to_string(),
            }),
            experimental: Some(ExperimentalConfig {
                clash_api: Some(ClashApiConfig {
                    external_controller: "127.0.0.1:9090".to_string(),
                    secret: None,
                }),
            }),
            dns: None,
            inbounds: vec![],
            outbounds: vec![],
            route: None,
        }
    }
}
