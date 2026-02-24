# singboxer

A terminal UI (TUI) application for managing sing-box configurations on servers. Parse Clash/Shadowsocks/V2Ray subscription links and control sing-box directly from the terminal.

## Table of Contents

- [How-To Guide](#how-to-guide)
- [Download Binaries](#download-binaries)
- [Upload to Server](#upload-to-server)
- [Complete Setup Walkthrough](#complete-setup-walkthrough)
- [Usage](#usage)
- [CLI Commands](#cli-commands)
- [Running as a Service](#running-as-a-service)
- [Troubleshooting](#troubleshooting)

---

## How-To Guide

This guide walks you through setting up singboxer and sing-box on your server to start using a proxy.

### Overview

You will:
1. Download sing-box and singboxer binaries
2. Upload them to your server
3. Import your subscription (from URL or file)
4. Select and test proxies
5. Start sing-box with your selected proxy
6. Use the proxy (transparent TUN mode)

**End Result:** All your server traffic goes through the selected proxy. You can switch proxies anytime without restarting.

---

## Download Binaries

### 1. Download sing-box

Go to [sing-box releases](https://github.com/SagerNet/sing-box/releases) and download the version for your server:

| Platform | Download Command |
|----------|-----------------|
| Linux AMD64 | `wget https://github.com/SagerNet/sing-box/releases/download/v1.11.7/sing-box-1.11.7-linux-amd64.tar.gz` |
| Linux ARM64 | `wget https://github.com/SagerNet/sing-box/releases/download/v1.11.7/sing-box-1.11.7-linux-arm64.tar.gz` |
| macOS AMD64 | `wget https://github.com/SagerNet/sing-box/releases/download/v1.11.7/sing-box-1.11.7-darwin-amd64.tar.gz` |
| macOS ARM64 | `wget https://github.com/SagerNet/sing-box/releases/download/v1.11.7/sing-box-1.11.7-darwin-arm64.tar.gz` |

Extract:
```bash
tar -xzf sing-box-1.11.7-linux-amd64.tar.gz
# Binary is at: sing-box-1.11.7-linux-amd64/sing-box
```

### 2. Build singboxer

```bash
# Clone or navigate to the repo
cd singboxer

# Build release version
cargo build --release

# Binary is at: target/release/singboxer
```

---

## Upload to Server

### Option 1: Using scp (from your local machine)

```bash
# Upload both binaries
scp target/release/singboxer user@your-server:/home/user/singbox/
scp sing-box-1.11.7-linux-amd64/sing-box user@your-server:/home/user/singbox/
```

### Option 2: Direct download on server

```bash
# SSH into your server
ssh user@your-server

# Create directory
mkdir -p ~/singbox
cd ~/singbox

# Download sing-box directly
wget https://github.com/SagerNet/sing-box/releases/download/v1.11.7/sing-box-1.11.7-linux-amd64.tar.gz
tar -xzf sing-box-1.11.7-linux-amd64.tar.gz
mv sing-box-1.11.7-linux-amd64/sing-box sing-box

# Upload singboxer via scp (or build on server if you have Rust)
# From local machine:
scp target/release/singboxer user@your-server:~/singbox/
```

---

## Complete Setup Walkthrough

### Step 1: Prepare Your Server

```bash
# SSH into your server
ssh user@your-server

# Navigate to singbox directory
cd ~/singbox

# Make binaries executable
chmod +x singbox singboxer

# Verify they work
./sing-box version
./singboxer --help
```

**Expected output:**
```
sing-box 1.11.7 ...
```

### Step 2: Import Your Subscription

You have two options: import from URL or from a file.

#### Option A: Import from Subscription URL

```bash
# Example: Clash subscription
./singboxer add "MyProvider" "https://my-provider.com/clash"

# Example: Shadowsocks subscription
./singboxer add "MySS" "https://my-ss.com/sub"
```

#### Option B: Import from File

If you have a subscription file (Clash YAML, etc.):

```bash
# Upload your file first
scp ~/Downloads/config.yaml user@server:~/singbox/

# On the server, import it
./singboxer import "MyProvider" "config.yaml"
```

Or from the TUI:
```bash
./singboxer

# Press 'i' to import (coming soon)
# For now, use CLI command above
```

### Step 3: Launch singboxer TUI

```bash
./singboxer
```

You'll see:
```
┌─────────────────────────────────────────┐
│     singboxer - sing-box: Stopped      │
├──────────────────┬──────────────────────┤
│ Subscriptions    │ Proxies              │
│ [l]              │ [p]                  │
│                  │                      │
│ MyProvider       │ (no proxies loaded)  │
│                  │                      │
└──────────────────┴──────────────────────┘
│ a:add s:save S:start r:reload t:test q:quit ?:help
└─────────────────────────────────────────┘
```

### Step 4: Load Your Proxies

1. Use arrow keys to select your subscription
2. Press `Enter` to load proxies
3. The Proxies panel will populate with available servers

### Step 5: Test Proxy Latencies

```text
Press: t
```

This tests all proxies concurrently and shows:
- 🟢 Green: < 100ms (fast)
- 🟡 Yellow: 100-300ms (good)
- 🟠 Orange: 300-1000ms (slow)
- 🔴 Red: > 1000ms or timeout

### Step 6: Select a Proxy

1. Navigate to the Proxies panel (press `→` or click)
2. Use arrow keys to select the fastest proxy
3. Note the proxy name (you'll activate it next)

### Step 7: Start sing-box

```text
Press: S
```

This:
1. Generates a sing-box config with your selected proxy
2. Starts sing-box in TUN mode (transparent proxy)
3. Header shows: `sing-boxer - sing-box: Running (PID: xxx)`

**Your server is now using the proxy!** All traffic (except local) goes through it.

### Step 8: Verify It's Working

```bash
# Check your IP (should show proxy location)
curl https://ipinfo.io/ip

# Check sing-box is running
ps aux | grep sing-box

# Or use singboxer status
# The header shows running state
```

### Step 9: Switch Proxies (Anytime!)

1. Select a different proxy in the Proxies panel
2. Press `Enter`
3. sing-box switches instantly (no restart needed)

### Step 10: Stop When Done

```text
Press: x
```

Or quit singboxer:
```text
Press: q
```

---

## Using sing-box Directly

Once you have a config generated, you can also run sing-box manually:

### Generated Config Location

```bash
~/.config/singboxer/singbox/config.json
```

### Manual Execution

```bash
# Run sing-box with generated config
./sing-box run -c ~/.config/singboxer/singbox/config.json

# Run in background
nohup ./sing-box run -c ~/.config/singboxer/singbox/config.json &

# Check logs
tail -f ~/.local/var/log/sing-box/*
```

---

## CLI Commands

All available CLI commands:

```bash
# Launch TUI (default)
./singboxer

# Add subscription from URL
./singboxer add "Name" "https://subscription-url"

# Import subscription from file
./singboxer import "Name" "/path/to/config.yaml"

# List subscriptions
./singboxer list

# Remove subscription
./singboxer remove "Name"

# Fetch and show proxies from URL (without adding)
./singboxer fetch "https://subscription-url"

# Generate config to file (without starting)
./singboxer generate "https://subscription-url" -o myconfig.json

# Show help
./singboxer --help
```

---

## TUI Key Bindings

| Key | Action |
|-----|--------|
| **Navigation** | |
| `Tab` / `←` / `→` | Switch panels (Subscriptions ↔ Proxies) |
| `↑` / `↓` | Navigate items in current panel |
| **Subscription Actions** | |
| `Enter` | Load proxies from selected subscription |
| `r` | Reload/refresh selected subscription |
| `d` | Delete selected subscription |
| **Proxy Actions** | |
| `Enter` | Activate selected proxy (requires sing-box running) |
| `t` | Test **all** proxy latencies |
| `T` | Test **selected** proxy latency |
| **sing-box Control** | |
| `S` | Start sing-box with selected proxy |
| `x` | Stop sing-box |
| `R` | Restart sing-box with new config |
| **Config** | |
| `s` | Save config to file (without starting) |
| **Other** | |
| `?` / `Esc` | Show/hide help popup |
| `q` | Quit singboxer |

---

## Running as a Service

To auto-start sing-box on boot:

### Create systemd Service

```bash
sudo nano /etc/systemd/system/singbox.service
```

Add this content:

```ini
[Unit]
Description=sing-box Proxy Service
After=network.target

[Service]
Type=simple
User=your-username
WorkingDirectory=/home/your-username/singbox
ExecStart=/home/your-username/singbox/sing-box run -c /home/your-username/.config/singboxer/singbox/config.json
Restart=on-failure
RestartSec=5s

# Required for TUN mode
CapabilityBoundingSet=CAP_NET_ADMIN CAP_NET_RAW
AmbientCapabilities=CAP_NET_ADMIN CAP_NET_RAW

[Install]
WantedBy=multi-user.target
```

### Enable and Start

```bash
sudo systemctl daemon-reload
sudo systemctl enable singbox
sudo systemctl start singbox

# Check status
sudo systemctl status singbox

# View logs
sudo journalctl -u singbox -f
```

### Using singboxer with systemd

1. Use singboxer to update config (`s` to save)
2. Restart service: `sudo systemctl restart singbox`
3. Or use singboxer's built-in control if Clash API is enabled

---

## Subscription File Import

singboxer can import subscriptions from local files:

### Supported File Formats

- **Clash YAML** (`.yaml`, `.yml`) - Most common
- **Shadowsocks JSON** (`.json`)
- **V2RayN URI list** (`.txt`) - One proxy per line (vmess://, vless://, etc.)

### Import Methods

**Method 1: CLI (Recommended)**
```bash
# Import a Clash config
./singboxer import "MyProvider" "/path/to/clash_config.yaml"

# Import a Shadowsocks list
./singboxer import "MySS" "/path/to/ss_list.txt"
```

**Method 2: Manual Configuration**

Edit `~/.config/singboxer/subscriptions.json`:

```json
[
  {
    "name": "MyProvider",
    "url": "file:///home/user/singbox/config.yaml",
    "type": "auto",
    "enabled": true
  }
]
```

Then reload in TUI (press `r` on the subscription).

### Upload and Import Workflow

```bash
# On your local machine
scp ~/Downloads/my-provider.yaml user@server:~/singbox/

# On the server
cd ~/singbox
./singboxer import "MyProvider" "my-provider.yaml"

# Launch TUI
./singboxer
# Select subscription, press Enter to load
```

---

## Configuration Files

All data stored in `~/.config/singboxer/`:

```
~/.config/singboxer/
├── subscriptions.json    # Your subscriptions (URL and file-based)
└── singbox/
    └── config.json       # Generated sing-box config
```

### subscriptions.json Format

```json
[
  {
    "name": "MyProvider",
    "url": "https://example.com/clash",
    "type": "clash",
    "enabled": true
  },
  {
    "name": "LocalFile",
    "url": "file:///home/user/config.yaml",
    "type": "auto",
    "enabled": true
  }
]
```

---

## Troubleshooting

### "sing-box not found"

**Error:** `sing-box: Not Installed` in header

**Solutions:**
```bash
# Check if sing-box is in PATH
which sing-box

# If not, add to PATH
export PATH=$PATH:~/singbox

# Or create symlink
sudo ln -s ~/singbox/sing-box /usr/local/bin/sing-box
```

### "Permission denied" with TUN mode

**Error:** `failed to initialize tun`

**Solutions:**
```bash
# Run with sudo
sudo ./sing-box run -c config.json

# Or add capabilities (recommended)
sudo setcap cap_net_admin,cap_net_raw+ep ~/singbox/sing-box
```

### "Can't connect to Clash API"

**Error:** Proxy switching fails

**Solutions:**
1. Make sure sing-box is running
2. Check Clash API is enabled in generated config
3. Verify API port (default: 9090) is not blocked

### Subscription not loading

**Check:**
```bash
# Test the URL directly
curl -L "your-subscription-url"

# For files, check path
ls -la /path/to/file

# Check singboxer logs
RUST_LOG=debug ./singboxer
```

---

## Features

### Supported Subscription Formats

| Format | Extensions | Import Method |
|--------|------------|----------------|
| Clash YAML | `.yaml`, `.yml` | URL, file |
| Shadowsocks | `.json` | URL, file |
| V2RayN URIs | `.txt` | URL, file |
| Base64 | Any | URL (auto-detected) |

### Supported Proxy Types

- Shadowsocks (SS)
- VMess
- VLESS (with Reality)
- Trojan
- Hysteria2 / H2
- SOCKS5
- HTTP/HTTPS
- TUIC

### Generated Config Features

- **TUN inbound** - Transparent proxy with auto-routing
- **SOCKS5 inbound** - Local proxy on `127.0.0.1:7890`
- **HTTP inbound** - Local proxy on `127.0.0.1:7891`
- **Selector outbound** - Manual proxy selection
- **URLTest outbound** - Auto-select fastest proxy
- **Clash API** - Control via dashboards (port 9090)
- **Routing rules** - Direct connection for private/CN IPs/domains

### Latency Testing

- 🟢 Green: < 100ms (fast)
- 🟡 Yellow: 100-300ms (good)
- 🟠 Orange: 300-1000ms (slow)
- 🔴 Red: > 1000ms or timeout
- Tests up to 5 proxies concurrently

---

## Quick Reference Card

```
┌─────────────────────────────────────────────┐
│  SINGBOXER QUICK START                      │
├─────────────────────────────────────────────┤
│  1. Add sub:  ./singboxer add "Name" "url"  │
│  2. Import:    ./singboxer import "Name" file│
│  3. Launch:    ./singboxer                   │
│  4. Load:      Select sub, Enter            │
│  5. Test:      Press t                      │
│  6. Start:     Press S                      │
│  7. Switch:    Select proxy, Enter          │
│  8. Stop:      Press x                      │
│  9. Quit:      Press q                      │
├─────────────────────────────────────────────┤
│  Generated config:                          │
│  ~/.config/singboxer/singbox/config.json   │
│                                             │
│  Run manually:                               │
│  ./sing-box run -c config.json              │
└─────────────────────────────────────────────┘
```

---

## License

MIT
