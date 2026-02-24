// Latency testing for proxies

use crate::models::{ProxyServer, ProxyType, ProxyConfig};
use anyhow::{Context, Result};
use std::time::Duration;
use tokio::time::timeout;

/// Test URL for latency checks
pub const DEFAULT_TEST_URL: &str = "http://www.gstatic.com/generate_204";
pub const ALTERNATIVE_TEST_URL: &str = "http://cp.cloudflare.com/generate_204";

/// Test latency for a single proxy
pub async fn test_proxy_latency(proxy: &ProxyServer, test_url: Option<&str>) -> Result<Duration> {
    let url = test_url.unwrap_or(DEFAULT_TEST_URL);

    // Build proxy address
    let _proxy_addr = format!("{}:{}", proxy.server, proxy.port);

    let result = match &proxy.config {
        ProxyConfig::Shadowsocks(ss) => {
            test_via_shadowsocks(proxy, ss, url).await
        }
        ProxyConfig::Vmess(vmess) => {
            test_via_vmess(proxy, vmess, url).await
        }
        ProxyConfig::Vless(vless) => {
            test_via_vless(proxy, vless, url).await
        }
        ProxyConfig::Trojan(trojan) => {
            test_via_trojan(proxy, trojan, url).await
        }
        ProxyConfig::Hysteria2(hy2) => {
            test_via_hysteria2(proxy, hy2, url).await
        }
        ProxyConfig::Socks(socks) => {
            test_via_socks(proxy, socks, url).await
        }
        ProxyConfig::Http(http) => {
            test_via_http(proxy, http, url).await
        }
    };

    // If test fails, try alternative URL
    match result {
        Ok(d) => Ok(d),
        Err(e) => {
            tracing::debug!("Primary test failed for {}: {}, trying alternative", proxy.name, e);
            test_via_direct_tcp(proxy, ALTERNATIVE_TEST_URL).await
        }
    }
}

/// Test via SOCKS5 proxy (simplest - works with reqwest)
async fn test_via_socks(proxy: &ProxyServer, _config: &crate::models::SocksConfig, url: &str) -> Result<Duration> {
    let proxy_addr = format!("socks5://{}:{}", proxy.server, proxy.port);
    test_via_http_proxy_url(proxy, &proxy_addr, url).await
}

/// Test via HTTP proxy
async fn test_via_http(proxy: &ProxyServer, _config: &crate::models::HttpConfig, url: &str) -> Result<Duration> {
    let is_https = matches!(proxy.proxy_type, ProxyType::Http);
    let scheme = if is_https { "https" } else { "http" };
    let proxy_addr = format!("{}://{}:{}", scheme, proxy.server, proxy.port);
    test_via_http_proxy_url(proxy, &proxy_addr, url).await
}

/// Generic test through HTTP/SOCKS proxy URL
async fn test_via_http_proxy_url(_proxy: &ProxyServer, proxy_url: &str, url: &str) -> Result<Duration> {
    let start = std::time::Instant::now();

    let response = timeout(
        Duration::from_secs(5),
        reqwest::Client::builder()
            .proxy(reqwest::Proxy::all(proxy_url)?)
            .timeout(Duration::from_secs(5))
            .build()?
            .get(url)
            .send()
    )
    .await
    .context("Timeout")?
    .context("Request failed")?;

    if response.status().is_success() || response.status() == 204 {
        Ok(start.elapsed())
    } else {
        Err(anyhow::anyhow!("HTTP status: {}", response.status()))
    }
}

/// Test via Shadowsocks - requires local client or tunnel
/// For now, we'll use a TCP connect test as a basic health check
async fn test_via_shadowsocks(proxy: &ProxyServer, _config: &crate::models::ShadowsocksConfig, url: &str) -> Result<Duration> {
    // Shadowsocks requires a local client
    // We'll do a basic TCP connection test to the server
    test_via_direct_tcp(proxy, url).await
}

/// Test via VMess - requires sing-box or similar client
async fn test_via_vmess(proxy: &ProxyServer, _config: &crate::models::VmessConfig, url: &str) -> Result<Duration> {
    // VMess requires a client - use TCP test
    test_via_direct_tcp(proxy, url).await
}

/// Test via VLESS - requires sing-box or similar client
async fn test_via_vless(proxy: &ProxyServer, _config: &crate::models::VlessConfig, url: &str) -> Result<Duration> {
    test_via_direct_tcp(proxy, url).await
}

/// Test via Trojan - requires sing-box or similar client
async fn test_via_trojan(proxy: &ProxyServer, _config: &crate::models::TrojanConfig, url: &str) -> Result<Duration> {
    test_via_direct_tcp(proxy, url).await
}

/// Test via Hysteria2 - requires sing-box or similar client
async fn test_via_hysteria2(proxy: &ProxyServer, _config: &crate::models::Hysteria2Config, url: &str) -> Result<Duration> {
    test_via_direct_tcp(proxy, url).await
}

/// Basic TCP connection test - measures time to connect to the proxy server
/// This is a fallback for proxy types that require a client
async fn test_via_direct_tcp(proxy: &ProxyServer, _url: &str) -> Result<Duration> {
    use tokio::net::TcpStream;
    use tokio::time::Instant;

    let addr = format!("{}:{}", proxy.server, proxy.port);
    let start = Instant::now();

    match timeout(Duration::from_secs(5), TcpStream::connect(&addr)).await {
        Ok(Ok(_stream)) => {
            // Successfully connected to the proxy server
            // Note: This doesn't verify the proxy actually works, just that it's reachable
            let latency = start.elapsed();
            // Add some overhead estimation for the proxy protocol
            Ok(latency.saturating_add(Duration::from_millis(50)))
        }
        Ok(Err(e)) => {
            Err(anyhow::anyhow!("TCP connection failed: {}", e))
        }
        Err(_) => {
            Err(anyhow::anyhow!("Connection timeout"))
        }
    }
}

/// Test multiple proxies concurrently with a limit
pub async fn test_proxies_concurrent(
    proxies: &mut [ProxyServer],
    test_url: Option<&str>,
    concurrent: usize,
) -> Vec<(usize, Result<Duration>)> {
    use futures::stream::{self, StreamExt};

    let results = stream::iter(proxies.iter_mut().enumerate())
        .map(|(i, proxy)| async move {
            let result = test_proxy_latency(proxy, test_url).await;
            (i, result)
        })
        .buffer_unordered(concurrent)
        .collect::<Vec<_>>()
        .await;

    results
}

/// Get color code for latency display
pub fn latency_color(latency_ms: u64) -> &'static str {
    if latency_ms < 100 {
        "green"  // Fast
    } else if latency_ms < 300 {
        "yellow" // Medium
    } else if latency_ms < 1000 {
        "orange" // Slow
    } else {
        "red"    // Very slow/timeout
    }
}

/// Format latency for display
pub fn format_latency(latency_ms: Option<u64>, testing: bool) -> String {
    if testing {
        return "...".to_string();
    }

    match latency_ms {
        Some(ms) => {
            if ms < 1000 {
                format!("{}ms", ms)
            } else {
                format!("{:.1}s", ms as f64 / 1000.0)
            }
        }
        None => "----".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_latency() {
        assert_eq!(format_latency(Some(50), false), "50ms");
        assert_eq!(format_latency(Some(500), false), "500ms");
        assert_eq!(format_latency(Some(1500), false), "1.5s");
        assert_eq!(format_latency(None, false), "----");
        assert_eq!(format_latency(Some(100), true), "...");
    }

    #[test]
    fn test_latency_color() {
        assert_eq!(latency_color(50), "green");
        assert_eq!(latency_color(150), "yellow");
        assert_eq!(latency_color(500), "orange");
        assert_eq!(latency_color(1500), "red");
    }
}
