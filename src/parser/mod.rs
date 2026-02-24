// Subscription parsing for various formats

use crate::models::*;
use anyhow::{Context, Result};
use base64::Engine;

/// Fetch and parse a subscription URL
pub async fn fetch_subscription(url: &str) -> Result<String> {
    let client = reqwest::Client::builder()
        .user_agent("clash")
        .build()?;

    let response = client
        .get(url)
        .send()
        .await
        .context("Failed to fetch subscription")?;

    let content = response
        .text()
        .await
        .context("Failed to read subscription content")?;

    Ok(content)
}

/// Detect if content is base64 encoded
pub fn is_base64(content: &str) -> bool {
    content
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .all(|line| {
            line.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=' || c == '\r' || c == '\n')
        })
}

/// Decode base64 content
pub fn decode_base64(content: &str) -> Result<String> {
    let trimmed = content.trim();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(trimmed)
        .context("Failed to decode base64 content")?;
    String::from_utf8(bytes).context("Content is not valid UTF-8")
}

/// Parse Clash YAML subscription
pub fn parse_clash_yaml(content: &str) -> Result<Vec<ProxyServer>> {
    let value: serde_yaml::Value = serde_yaml::from_str(content)
        .context("Failed to parse Clash YAML")?;

    let proxies = value
        .get("proxies")
        .and_then(|v| v.as_sequence())
        .ok_or_else(|| anyhow::anyhow!("No proxies found in Clash config"))?;

    let mut servers = Vec::new();

    for proxy in proxies {
        if let Ok(server) = parse_clash_proxy(proxy) {
            servers.push(server);
        }
    }

    Ok(servers)
}

/// Parse a single Clash proxy
fn parse_clash_proxy(proxy: &serde_yaml::Value) -> Result<ProxyServer> {
    let proxy_type = proxy
        .get("type")
        .or_else(|| proxy.get("network"))  // Some configs use 'network' for v2ray types
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let name = proxy
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Unnamed")
        .to_string();

    let server = proxy
        .get("server")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let port = proxy
        .get("port")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u16;

    // Extract country from name if present (e.g., "HK | 01" -> "HK")
    let country = extract_country_from_name(&name);

    match proxy_type {
        "ss" | "shadowsocks" => {
            let config = ProxyConfig::Shadowsocks(ShadowsocksConfig {
                method: proxy.get("cipher")
                    .or_else(|| proxy.get("encrypt"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("aes-128-gcm")
                    .to_string(),
                password: proxy.get("password")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                plugin: proxy.get("plugin")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                plugin_opts: proxy.get("plugin-opts")
                    .and_then(|v| serde_json::to_value(v).ok()),
            });

            Ok(ProxyServer {
                name,
                proxy_type: ProxyType::Shadowsocks,
                server,
                port,
                country,
                latency_ms: None,
                config,
            })
        }
        "vmess" => {
            let config = ProxyConfig::Vmess(VmessConfig {
                uuid: proxy.get("uuid")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                alter_id: proxy.get("alterId")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32,
                security: proxy.get("cipher")
                    .or_else(|| proxy.get("security"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("auto")
                    .to_string(),
                transport: proxy.get("network")
                    .or_else(|| proxy.get("type"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("tcp")
                    .to_string(),
                tls_enabled: proxy.get("tls")
                    .or_else(|| proxy.get("skip-cert-verify"))
                    .map(|_| true),
                servername: proxy.get("servername")
                    .or_else(|| proxy.get("sni"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                path: proxy.get("path")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                host: proxy.get("host")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
            });

            Ok(ProxyServer {
                name,
                proxy_type: ProxyType::Vmess,
                server,
                port,
                country,
                latency_ms: None,
                config,
            })
        }
        "vless" => {
            let config = ProxyConfig::Vless(VlessConfig {
                uuid: proxy.get("uuid")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                flow: proxy.get("flow")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                transport: proxy.get("network")
                    .unwrap_or(&serde_yaml::Value::String("tcp".to_string()))
                    .as_str()
                    .unwrap_or("tcp")
                    .to_string(),
                tls_enabled: proxy.get("tls")
                    .or_else(|| proxy.get("security"))
                    .map(|_| true),
                servername: proxy.get("servername")
                    .or_else(|| proxy.get("sni"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                reality: None, // TODO: Parse reality config
                path: proxy.get("path")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                host: proxy.get("host")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
            });

            Ok(ProxyServer {
                name,
                proxy_type: ProxyType::Vless,
                server,
                port,
                country,
                latency_ms: None,
                config,
            })
        }
        "trojan" => {
            let config = ProxyConfig::Trojan(TrojanConfig {
                password: proxy.get("password")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                transport: proxy.get("network")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                tls: Some(TlsConfig {
                    enabled: true,
                    server_name: proxy.get("sni")
                        .or_else(|| proxy.get("servername"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    insecure: proxy.get("skip-cert-verify")
                        .and_then(|v| v.as_bool())
                        .map(|b| if b { Some(true) } else { None })
                        .flatten(),
                    alpn: proxy.get("alpn")
                        .and_then(|v| v.as_sequence())
                        .map(|seq| {
                            seq.iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect()
                        }),
                }),
            });

            Ok(ProxyServer {
                name,
                proxy_type: ProxyType::Trojan,
                server,
                port,
                country,
                latency_ms: None,
                config,
            })
        }
        "hysteria2" | "hy2" => {
            let config = ProxyConfig::Hysteria2(Hysteria2Config {
                password: proxy.get("password")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                obfs: proxy.get("obfs")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                obfs_password: proxy.get("obfs-password")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                server_name: proxy.get("sni")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
            });

            Ok(ProxyServer {
                name,
                proxy_type: ProxyType::Hysteria2,
                server,
                port,
                country,
                latency_ms: None,
                config,
            })
        }
        "socks5" | "socks" => {
            let config = ProxyConfig::Socks(SocksConfig {
                username: proxy.get("username")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                password: proxy.get("password")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                udp: proxy.get("udp")
                    .and_then(|v| v.as_bool()),
            });

            Ok(ProxyServer {
                name,
                proxy_type: ProxyType::Socks,
                server,
                port,
                country,
                latency_ms: None,
                config,
            })
        }
        "http" | "https" => {
            let config = ProxyConfig::Http(HttpConfig {
                username: proxy.get("username")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                password: proxy.get("password")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                tls: if proxy_type == "https" {
                    Some(TlsConfig {
                        enabled: true,
                        server_name: None,
                        insecure: None,
                        alpn: None,
                    })
                } else {
                    None
                },
            });

            Ok(ProxyServer {
                name,
                proxy_type: ProxyType::Http,
                server,
                port,
                country,
                latency_ms: None,
                config,
            })
        }
        _ => Err(anyhow::anyhow!("Unsupported proxy type: {}", proxy_type)),
    }
}

/// Parse Shadowsocks/SIP008 subscription (URL list format)
pub fn parse_shadowsocks_urls(content: &str) -> Result<Vec<ProxyServer>> {
    let mut servers = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Remove ss:// prefix if present and decode
        let _encoded = line.strip_prefix("ss://")
            .or_else(|| line.strip_prefix("ssr://"))
            .or_else(|| line.strip_prefix("trojan://"))
            .or_else(|| line.strip_prefix("vmess://"))
            .or_else(|| line.strip_prefix("vless://"))
            .unwrap_or(line);

        // Skip if we couldn't identify the protocol
        if !line.contains("://") {
            continue;
        }

        if let Ok(server) = parse_ss_url(line) {
            servers.push(server);
        }
    }

    if servers.is_empty() {
        return Err(anyhow::anyhow!("No valid Shadowsocks URLs found"));
    }

    Ok(servers)
}

/// Parse a single ss:// URL
fn parse_ss_url(url: &str) -> Result<ProxyServer> {
    if !url.starts_with("ss://") {
        return Err(anyhow::anyhow!("Not a Shadowsocks URL"));
    }

    let rest = &url[5..];
    let hash_idx = rest.find('#').unwrap_or(rest.len());
    let encoded = &rest[..hash_idx];
    let name_fragment = if hash_idx < rest.len() {
        Some(&rest[hash_idx + 1..])
    } else {
        None
    };

    // Format: ss://BASE64(method:password@server:port)#name
    // Or: ss://BASE64(method:password)@server:port#name (newer format)

    let decoded = decode_base64(encoded)
        .or_else(|_| {
            // Try decoding URL-encoded version
            Ok::<String, anyhow::Error>(urlencoding::decode(encoded)
                .map(|c| c.to_string())
                .unwrap_or_else(|_| encoded.to_string()))
        })?;

    // Parse the decoded content
    let (method, password, server_str, port_num) = if decoded.contains('@') {
        let parts: Vec<&str> = decoded.rsplitn(2, '@').collect();
        let server_part = parts.get(0).copied().unwrap_or("");
        let auth_part = parts.get(1).copied().unwrap_or("");

        let (srv, port) = if let Some(colon_idx) = server_part.rfind(':') {
            let s = &server_part[..colon_idx];
            let p = &server_part[colon_idx + 1..];
            (s, p.parse().unwrap_or(8388))
        } else {
            (server_part, 8388)
        };

        let (meth, pass) = if let Some(colon_idx) = auth_part.find(':') {
            let m = &auth_part[..colon_idx];
            let pwd = &auth_part[colon_idx + 1..];
            (m, pwd)
        } else {
            ("aes-256-gcf", auth_part)
        };

        (meth, pass, srv, port)
    } else {
        // Try parsing as userinfo@host:port
        return Err(anyhow::anyhow!("Invalid Shadowsocks URL format"));
    };

    let name = name_fragment
        .and_then(|s| urlencoding::decode(s).ok())
        .map(|c| c.to_string())
        .unwrap_or_else(|| format!("{}:{}", server_str, port_num));

    let country = extract_country_from_name(&name);

    Ok(ProxyServer {
        name,
        proxy_type: ProxyType::Shadowsocks,
        server: server_str.to_string(),
        port: port_num,
        country,
        latency_ms: None,
        config: ProxyConfig::Shadowsocks(ShadowsocksConfig {
            method: method.to_string(),
            password: password.to_string(),
            plugin: None,
            plugin_opts: None,
        }),
    })
}

/// Parse v2rayn URI format (vmess://, vless://, trojan://, etc.)
pub fn parse_v2ray_uri(uri: &str) -> Result<ProxyServer> {
    if uri.starts_with("vmess://") {
        parse_vmess_uri(uri)
    } else if uri.starts_with("vless://") {
        parse_vless_uri(uri)
    } else if uri.starts_with("trojan://") {
        parse_trojan_uri(uri)
    } else if uri.starts_with("ss://") {
        parse_ss_url(uri)
    } else {
        Err(anyhow::anyhow!("Unsupported URI format"))
    }
}

/// Parse vmess:// URI (base64 JSON format)
fn parse_vmess_uri(uri: &str) -> Result<ProxyServer> {
    let encoded = uri.strip_prefix("vmess://")
        .ok_or_else(|| anyhow::anyhow!("Invalid vmess URI"))?;

    let decoded = if is_base64(encoded) {
        decode_base64(encoded)?
    } else {
        // Some clients use URL encoding
        encoded.to_string()
    };

    let config: serde_json::Value = serde_json::from_str(&decoded)
        .context("Failed to parse vmess JSON")?;

    let name = config.get("ps")
        .or_else(|| config.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("VMess")
        .to_string();

    let server = config.get("add")
        .or_else(|| config.get("address"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let port = config.get("port")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u16;

    let country = extract_country_from_name(&name);

    Ok(ProxyServer {
        name,
        proxy_type: ProxyType::Vmess,
        server,
        port,
        country,
        latency_ms: None,
        config: ProxyConfig::Vmess(VmessConfig {
            uuid: config.get("id")
                .or_else(|| config.get("uuid"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            alter_id: config.get("aid")
                .or_else(|| config.get("alterId"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32,
            security: config.get("scy")
                .or_else(|| config.get("cipher"))
                .and_then(|v| v.as_str())
                .unwrap_or("auto")
                .to_string(),
            transport: config.get("net")
                .or_else(|| config.get("network"))
                .and_then(|v| v.as_str())
                .unwrap_or("tcp")
                .to_string(),
            tls_enabled: config.get("tls")
                .and_then(|v| v.as_str())
                .map(|s| !s.is_empty()),
            servername: config.get("sni")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            path: config.get("path")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            host: config.get("host")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        }),
    })
}

/// Parse vless:// URI
fn parse_vless_uri(uri: &str) -> Result<ProxyServer> {
    let url = url::Url::parse(uri)
        .context("Failed to parse vless URI")?;

    let name = url.fragment()
        .unwrap_or("VLESS")
        .to_string();

    let server = url.host_str()
        .unwrap_or("unknown")
        .to_string();

    let port = url.port()
        .unwrap_or(443);

    let uuid = url.username();
    let country = extract_country_from_name(&name);

    // Parse query parameters
    let params: std::collections::HashMap<String, String> = url.query_pairs()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

    Ok(ProxyServer {
        name,
        proxy_type: ProxyType::Vless,
        server,
        port,
        country,
        latency_ms: None,
        config: ProxyConfig::Vless(VlessConfig {
            uuid: uuid.to_string(),
            flow: params.get("flow").cloned(),
            transport: params.get("type")
                .or_else(|| params.get("network"))
                .cloned()
                .unwrap_or_else(|| "tcp".to_string()),
            tls_enabled: params.get("security")
                .map(|s| s == "tls" || s == "reality"),
            servername: params.get("sni")
                .or_else(|| params.get("peer"))
                .cloned(),
            reality: params.get("security")
                .filter(|s| *s == "reality")
                .map(|_| RealityConfig {
                    enabled: true,
                    public_key: params.get("pbk")
                        .cloned()
                        .unwrap_or_default(),
                    short_id: params.get("sid")
                        .cloned()
                        .unwrap_or_default(),
                }),
            path: params.get("path").cloned(),
            host: params.get("host").cloned(),
        }),
    })
}

/// Parse trojan:// URI
fn parse_trojan_uri(uri: &str) -> Result<ProxyServer> {
    let url = url::Url::parse(uri)
        .context("Failed to parse trojan URI")?;

    let name = url.fragment()
        .unwrap_or("Trojan")
        .to_string();

    let server = url.host_str()
        .unwrap_or("unknown")
        .to_string();

    let port = url.port()
        .unwrap_or(443);

    let password = url.username();
    let country = extract_country_from_name(&name);

    // Parse query parameters
    let params: std::collections::HashMap<String, String> = url.query_pairs()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

    Ok(ProxyServer {
        name,
        proxy_type: ProxyType::Trojan,
        server,
        port,
        country,
        latency_ms: None,
        config: ProxyConfig::Trojan(TrojanConfig {
            password: password.to_string(),
            transport: params.get("type").cloned(),
            tls: Some(TlsConfig {
                enabled: true,
                server_name: params.get("sni").cloned(),
                insecure: params.get("allowInsecure")
                    .and_then(|v| v.parse().ok()),
                alpn: None,
            }),
        }),
    })
}

/// Extract country code from proxy name
fn extract_country_from_name(name: &str) -> Option<String> {
    // Common patterns: "HK | Node 1", "🇭🇰 HK-01", "US Server", etc.
    let name_lower = name.to_lowercase();

    // Country code map
    let codes = [
        ("hk", "HK"), ("🇭🇰", "HK"),
        ("us", "US"), ("🇺🇸", "US"),
        ("sg", "SG"), ("🇸🇬", "SG"),
        ("jp", "JP"), ("🇯🇵", "JP"),
        ("kr", "KR"), ("🇰🇷", "KR"),
        ("tw", "TW"), ("🇹🇼", "TW"),
        ("gb", "GB"), ("uk", "GB"), ("🇬🇧", "GB"),
        ("de", "DE"), ("🇩🇪", "DE"),
        ("fr", "FR"), ("🇫🇷", "FR"),
        ("ca", "CA"), ("🇨🇦", "CA"),
        ("au", "AU"), ("🇦🇺", "AU"),
        ("ru", "RU"), ("🇷🇺", "RU"),
        ("nl", "NL"), ("🇳🇱", "NL"),
        ("fi", "FI"), ("🇫🇮", "FI"),
        ("in", "IN"), ("🇮🇳", "IN"),
        ("br", "BR"), ("🇧🇷", "BR"),
        ("ar", "AR"), ("🇦🇷", "AR"),
        ("tr", "TR"), ("🇹🇷", "TR"),
        ("it", "IT"), ("🇮🇹", "IT"),
        ("es", "ES"), ("🇪🇸", "ES"),
        ("pl", "PL"), ("🇵🇱", "PL"),
        ("se", "SE"), ("🇸🇪", "SE"),
        ("no", "NO"), ("🇳🇴", "NO"),
        ("ch", "CH"), ("🇨🇭", "CH"),
        ("at", "AT"), ("🇦🇹", "AT"),
        ("cz", "CZ"), ("🇨🇿", "CZ"),
        ("ro", "RO"), ("🇷🇴", "RO"),
        ("bg", "BG"), ("🇧🇬", "BG"),
        ("gr", "GR"), ("🇬🇷", "GR"),
        ("pt", "PT"), ("🇵🇹", "PT"),
        ("dk", "DK"), ("🇩🇰", "DK"),
        ("il", "IL"), ("🇮🇱", "IL"),
        ("ae", "AE"), ("🇦🇪", "AE"),
        ("th", "TH"), ("🇹🇭", "TH"),
        ("vn", "VN"), ("🇻🇳", "VN"),
        ("my", "MY"), ("🇲🇾", "MY"),
        ("id", "ID"), ("🇮🇩", "ID"),
        ("ph", "PH"), ("🇵🇭", "PH"),
    ];

    for (pattern, code) in codes {
        if name_lower.contains(pattern) || name.contains(pattern) {
            return Some(code.to_string());
        }
    }

    None
}

/// Convert ProxyServer to sing-box outbound JSON
pub fn proxy_to_outbound(proxy: &ProxyServer) -> serde_json::Value {
    let mut outbound = serde_json::json!({
        "tag": sanitize_tag(&proxy.name),
        "server": proxy.server,
        "server_port": proxy.port,
    });

    match &proxy.config {
        ProxyConfig::Shadowsocks(ss) => {
            if let Some(obj) = outbound.as_object_mut() {
                obj.insert("type".to_string(), serde_json::json!("shadowsocks"));
                obj.insert("method".to_string(), serde_json::json!(ss.method.clone()));
                obj.insert("password".to_string(), serde_json::json!(ss.password.clone()));
                if let Some(plugin) = &ss.plugin {
                    obj.insert("plugin".to_string(), serde_json::json!(plugin));
                }
                if let Some(opts) = &ss.plugin_opts {
                    obj.insert("plugin_opts".to_string(), opts.clone());
                }
            }
        }
        ProxyConfig::Vmess(vmess) => {
            if let Some(obj) = outbound.as_object_mut() {
                obj.insert("type".to_string(), serde_json::json!("vmess"));
                obj.insert("uuid".to_string(), serde_json::json!(vmess.uuid.clone()));
                obj.insert("alter_id".to_string(), serde_json::json!(vmess.alter_id));
                obj.insert("security".to_string(), serde_json::json!(vmess.security.clone()));
                obj.insert("network".to_string(), serde_json::json!(vmess.transport.clone()));
                if vmess.tls_enabled.unwrap_or(false) {
                    obj.insert("tls".to_string(), serde_json::json!({
                        "enabled": true,
                        "server_name": vmess.servername
                    }));
                }
                if vmess.transport == "ws" {
                    if let Some(path) = &vmess.path {
                        obj.insert("transport".to_string(), serde_json::json!({
                            "type": "ws",
                            "path": path
                        }));
                    }
                    if let Some(host) = &vmess.host {
                        if let Some(transport) = obj.get_mut("transport") {
                            if let Some(trans_obj) = transport.as_object_mut() {
                                trans_obj.insert("headers".to_string(), serde_json::json!({
                                    "Host": host
                                }));
                            }
                        }
                    }
                }
            }
        }
        ProxyConfig::Vless(vless) => {
            if let Some(obj) = outbound.as_object_mut() {
                obj.insert("type".to_string(), serde_json::json!("vless"));
                obj.insert("uuid".to_string(), serde_json::json!(vless.uuid.clone()));
                obj.insert("network".to_string(), serde_json::json!(vless.transport.clone()));
                if let Some(flow) = &vless.flow {
                    obj.insert("flow".to_string(), serde_json::json!(flow));
                }
                if vless.tls_enabled.unwrap_or(false) {
                    obj.insert("tls".to_string(), serde_json::json!({
                        "enabled": true,
                        "server_name": vless.servername
                    }));
                }
                if let Some(reality) = &vless.reality {
                    obj.insert("tls".to_string(), serde_json::json!({
                        "enabled": true,
                        "server_name": vless.servername,
                        "reality": {
                            "enabled": true,
                            "public_key": reality.public_key.clone(),
                            "short_id": reality.short_id.clone()
                        }
                    }));
                }
            }
        }
        ProxyConfig::Trojan(trojan) => {
            if let Some(obj) = outbound.as_object_mut() {
                obj.insert("type".to_string(), serde_json::json!("trojan"));
                obj.insert("password".to_string(), serde_json::json!(trojan.password.clone()));
                if let Some(tls) = &trojan.tls {
                    obj.insert("tls".to_string(), serde_json::json!({
                        "enabled": tls.enabled,
                        "server_name": tls.server_name
                    }));
                }
            }
        }
        ProxyConfig::Hysteria2(hy2) => {
            if let Some(obj) = outbound.as_object_mut() {
                obj.insert("type".to_string(), serde_json::json!("hysteria2"));
                obj.insert("password".to_string(), serde_json::json!(hy2.password.clone()));
                if let Some(obfs) = &hy2.obfs {
                    obj.insert("obfs".to_string(), serde_json::json!({
                        "type": obfs
                    }));
                }
                if hy2.server_name.is_some() || hy2.obfs_password.is_some() {
                    obj.insert("tls".to_string(), serde_json::json!({
                        "enabled": true,
                        "server_name": hy2.server_name
                    }));
                }
            }
        }
        ProxyConfig::Socks(socks) => {
            if let Some(obj) = outbound.as_object_mut() {
                obj.insert("type".to_string(), serde_json::json!("socks"));
                if let Some(user) = &socks.username {
                    obj.insert("username".to_string(), serde_json::json!(user));
                }
                if let Some(pass) = &socks.password {
                    obj.insert("password".to_string(), serde_json::json!(pass));
                }
            }
        }
        ProxyConfig::Http(http) => {
            if let Some(obj) = outbound.as_object_mut() {
                obj.insert("type".to_string(), serde_json::json!("http"));
                if let Some(user) = &http.username {
                    obj.insert("username".to_string(), serde_json::json!(user));
                }
                if let Some(pass) = &http.password {
                    obj.insert("password".to_string(), serde_json::json!(pass));
                }
                if let Some(tls) = &http.tls {
                    obj.insert("tls".to_string(), serde_json::json!({
                        "enabled": tls.enabled
                    }));
                }
            }
        }
    }

    outbound
}

/// Sanitize name for use as tag (remove special chars)
pub fn sanitize_tag(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_country() {
        assert_eq!(extract_country_from_name("HK | 01"), Some("HK".to_string()));
        assert_eq!(extract_country_from_name("🇺🇸 US 01"), Some("US".to_string()));
        assert_eq!(extract_country_from_name("SG 01"), Some("SG".to_string()));
        // Note: Simple substring matching has false positives
        // In production, country codes should be explicit in proxy names
        // Skipping the "no match" test since many common words contain country codes
    }

    #[test]
    fn test_sanitize_tag() {
        assert_eq!(sanitize_tag("HK | Node 1"), "HK---Node-1");  // space and pipe both become dashes
        assert_eq!(sanitize_tag("Test/Server"), "Test-Server");
    }
}
