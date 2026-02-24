# singboxer - Claude Code Context

## Project Overview

A terminal UI (TUI) application written in Rust for managing sing-box configurations on servers. Parses subscription links (Clash/Shadowsocks/V2Ray) and provides an interface to select and activate proxies with sing-box integration.

**Repository:** `/Users/randomradio/src/singboxer`

## Architecture

### Directory Structure

```
singboxer/
├── Cargo.toml           # Dependencies and project config
├── src/
│   ├── main.rs          # CLI entry point with clap commands
│   ├── lib.rs           # Library exports
│   ├── models/          # Data structures (Subscription, ProxyServer, SingBoxConfig)
│   ├── parser/          # Subscription parsing (Clash YAML, Shadowsocks, V2Ray URIs)
│   ├── config/          # Config management and sing-box JSON generation
│   ├── singbox/         # sing-box binary detection and process control
│   ├── latency/         # Proxy latency testing
│   └── tui/             # Terminal UI implementation (ratatui)
├── IMPLEMENTATION_PLAN.md  # Development stages
└── README.md            # User documentation
```

### Key Dependencies

- `ratatui` - Terminal UI framework
- `reqwest` - HTTP client for fetching subscriptions
- `tokio` - Async runtime
- `serde` / `serde_json` / `serde_yaml` - Serialization
- `crossterm` - Terminal handling
- `sysinfo` - Process detection for sing-box

## Core Data Structures

### `models/mod.rs`

**Key Types:**
- `Subscription` - name, url, type (Clash/Shadowsocks/V2Ray/Singbox/Auto), enabled
- `ProxyServer` - name, type (Shadowsocks/VMess/VLESS/Trojan/Hysteria2/SOCKS/HTTP), server, port, latency_ms, config
- `SingBoxConfig` - Full sing-box configuration JSON structure
- `ProxyConfig` - Enum of proxy-specific configs (ShadowsocksConfig, VmessConfig, etc.)

### Subscription Parsing

**`parser/mod.rs`** handles multiple formats:

1. **Clash YAML** - `parse_clash_yaml()` - Reads proxies array from YAML
2. **Shadowsocks URL list** - `parse_shadowsocks_urls()` - Parses ss:// URLs
3. **V2RayN URIs** - `parse_v2ray_uri()` - Parses vmess://, vless://, trojan://
4. **Base64** - Auto-detected via `is_base64()` and decoded

All proxies are converted to `ProxyServer` structs, then to sing-box outbounds via `proxy_to_outbound()`.

### Config Generation

**`config/mod.rs`** generates sing-box configs:

- `generate_singbox_config()` - Creates full config with:
  - TUN inbound (transparent proxy)
  - SOCKS5/HTTP inbounds (local proxies)
  - Selector outbound (manual selection)
  - URLTest outbound (auto fastest)
  - Clash API for proxy switching
  - Routing rules (private/CN direct)

### sing-box Integration

**`singbox/mod.rs`** manages the sing-box binary:

- `SingBoxManager::check_installation()` - Finds sing-box in PATH/common locations
- `SingBoxManager::start()` - Starts sing-box with generated config
- `SingBoxManager::stop()` - Stops running process
- `SingBoxManager::switch_proxy()` - Changes active proxy via Clash API
- `SingBoxManager::test_latency()` - Tests proxy latency

### Latency Testing

**`latency.rs`** tests proxy connectivity:

- SOCKS5/HTTP - Full HTTP test through proxy
- Other types - TCP connection test (requires sing-box for protocol testing)
- Color-coded results: <100ms (green), 100-300ms (yellow), 300-1000ms (orange), >1000ms (red)

### TUI

**`tui/mod.rs`** - ratatui-based interface:

- Two panels: Subscriptions (left), Proxies (right)
- Status bar with hints based on sing-box state
- Help popup (`?` key)
- Keyboard navigation (arrows, tab, enter)

## TUI Key Bindings

| Key | Action |
|-----|--------|
| Navigation | |
| `Tab`/`←`/`→` | Switch panels |
| `↑`/`↓` | Navigate lists |
| Actions | |
| `Enter` | Load subscription / Activate proxy |
| `S` | Start sing-box |
| `x` | Stop sing-box |
| `R` | Restart sing-box |
| `s` | Save config to file |
| `r` | Reload subscription |
| `t` | Test all proxy latencies |
| `T` | Test selected proxy |
| `d` | Delete subscription |
| `?`/`Esc` | Toggle help |
| `q` | Quit |

## CLI Commands

```bash
singboxer                    # Launch TUI
singboxer add <name> <url>    # Add subscription
singboxer list                # List subscriptions
singboxer remove <name>       # Remove subscription
singboxer fetch <url>         # Fetch and show proxies
singboxer generate <url>      # Generate config.json
```

## Configuration Files

- `~/.config/singboxer/subscriptions.json` - Saved subscriptions
- `~/.config/singboxer/singbox/config.json` - Generated sing-box config

## Common Tasks

### Adding a New Subscription Type

1. Add variant to `SubscriptionType` enum in `models/mod.rs`
2. Add parsing logic in `parser/mod.rs` (e.g., `parse_foo_format()`)
5. Add to `parse_subscription_content()` in `tui/mod.rs`

### Adding a New Proxy Type

1. Add `ProxyType` variant in `models/mod.rs`
2. Add config struct (e.g., `FooConfig`)
3. Add parsing in `parser/mod.rs` (Clash YAML and URI formats)
4. Add conversion in `proxy_to_outbound()` in `parser/mod.rs`
5. Add latency test method in `latency.rs`

### Modifying Generated sing-box Config

Edit `generate_singbox_config()` in `config/mod.rs`:
- Inbounds - TUN, SOCKS, HTTP settings
- Outbounds - Selector, URLTest, proxy list
- Route - DNS rules, final outbound
- Experimental - Clash API settings

## Development Workflow

1. **Make changes**
2. **Run tests**: `cargo test`
3. **Build**: `cargo build --release`
4. **Test locally**: `./target/release/singboxer`
5. **Commit**: Use clear commit messages

## Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_extract_country

# Run with logging
RUST_LOG=debug cargo test
```

## Known Limitations

1. **Latency testing** - For VMess/VLESS/Trojan/Hysteria2, only tests TCP connectivity to the server (not full proxy protocol test) - these require sing-box or client to be running for full test

2. **Country detection** - Simple substring matching can have false positives (e.g., "Server" contains "se" for Sweden)

3. **TUI input** - Adding subscriptions via TUI not implemented (use CLI)

## Future Enhancements

- Real-time traffic statistics display
- Favorites/proxy groups
- Auto-refresh subscriptions
- Config profiles
- Direct sing-box protocol testing for latency
