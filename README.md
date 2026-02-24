# singboxer

A terminal UI (TUI) application for managing sing-box configurations. Parse Clash/Shadowsocks/V2Ray subscription links and control sing-box directly from the terminal.

## Features

- **Multiple Subscription Formats**:
  - Clash YAML
  - Shadowsocks/SIP008 URL lists
  - V2RayN URI format (vmess://, vless://, trojan://)
  - Base64 encoded subscriptions

- **Built-in sing-box Management**:
  - Automatic sing-box binary detection
  - Start/stop/restart sing-box from TUI
  - Real-time status display
  - Proxy switching via Clash API (no restart needed)
  - Latency testing

- **TUI Interface**:
  - Browse and manage subscriptions
  - View and select proxies
  - Generate and apply configurations
  - Keyboard navigation

- **CLI Commands**:
  - Add/remove subscriptions
  - Fetch and display proxies
  - Generate config directly from URL

## Installation

### Prerequisites

Install sing-box first: https://github.com/SagerNet/sing-box#installation

```bash
# macOS (Homebrew)
brew install sing-box

# Linux (varies by distro)
# See sing-box GitHub for instructions
```

### Build singboxer

```bash
cargo build --release

# The binary will be at target/release/singboxer
# Optionally install to PATH
install target/release/singboxer ~/.local/bin/
```

## Usage

### Interactive TUI

```bash
singboxer
```

**Key bindings:**

| Key | Action |
|-----|--------|
| `Tab` / `←` / `→` | Switch panels |
| `↑` / `↓` | Navigate lists |
| `Enter` | Load subscription / Activate proxy |
| `S` | Start sing-box |
| `x` | Stop sing-box |
| `R` | Restart sing-box |
| `s` | Save config to file |
| `r` | Reload subscription |
| `t` | Test proxy latencies |
| `d` | Delete selected subscription |
| `?` / `Esc` | Toggle help |
| `q` | Quit |

### Workflow

1. **Add subscriptions** (via CLI or edit the config file):
   ```bash
   singboxer add "My Provider" "https://example.com/clash"
   ```

2. **Launch TUI** and load proxies:
   ```bash
   singboxer
   # Navigate to subscription, press Enter
   ```

3. **Select a proxy** and activate:
   - Navigate to Proxies panel
   - Select desired proxy
   - Press `Enter` to activate (if sing-box is running)

4. **Start sing-box** (press `S`):
   - Automatically generates config with selected proxy
   - Starts sing-box process
   - Status shows "Running (PID: ...)"

5. **Switch proxies** on-the-fly:
   - Just select a different proxy and press `Enter`
   - Uses Clash API - no restart needed

### CLI Commands

```bash
# Add a subscription
singboxer add "My Sub" "https://example.com/sub"

# List subscriptions
singboxer list

# Remove a subscription
singboxer remove "My Sub"

# Fetch and display proxies from a URL
singboxer fetch "https://example.com/sub"

# Generate config from a subscription URL
singboxer generate "https://example.com/sub" -o config.json
```

## Configuration

- **Subscriptions**: `~/.config/singboxer/subscriptions.json`
- **Generated configs**: `~/.config/singboxer/singbox/config.json`

## Generated Config Features

The generated sing-box configuration includes:

- **TUN inbound** - Transparent proxy mode with auto-routing
- **SOCKS inbound** - Local SOCKS5 proxy on `127.0.0.1:7890`
- **HTTP inbound** - Local HTTP proxy on `127.0.0.1:7891`
- **Selector outbound** - Manual proxy selection (via Clash API)
- **URLTest outbound** - Auto-select fastest proxy
- **Clash API** - Control via compatible dashboards (Yacd, MetaCubeXD)
- **Routing rules** - Direct connection for private/CN IPs/domains

## Proxy Type Support

- Shadowsocks
- VMess
- VLESS (with Reality support)
- Trojan
- Hysteria2
- SOCKS5
- HTTP/HTTPS

## sing-box Integration

When sing-box is not found, singboxer will display:
```
sing-box: Not Installed
```

And provide installation instructions in the help (`?` key).

When sing-box is running, the header shows:
```
sing-boxer - sing-box: Running (PID: 12345)
```

## License

MIT
