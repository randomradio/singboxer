# singboxer

A terminal UI (TUI) application for managing sing-box configurations on servers. Parse Clash/Shadowsocks/V2Ray subscription links and control sing-box directly from the terminal.

## Table of Contents

- [Quick Start (Server Deployment)](#quick-start-server-deployment)
- [Download Binaries](#download-binaries)
- [Upload to Server](#upload-to-server)
- [Usage](#usage)
- [CLI Commands](#cli-commands)
- [Running as a Service](#running-as-a-service)

---

## Quick Start (Server Deployment)

### Step 1: Download sing-box Binary

Go to the [sing-box releases](https://github.com/SagerNet/sing-box/releases) page and download the appropriate version for your server:

```bash
# For Linux AMD64 (most servers)
wget https://github.com/SagerNet/sing-box/releases/download/v1.11.7/sing-box-1.11.7-linux-amd64.tar.gz

# Extract
tar -xzf sing-box-1.11.7-linux-amd64.tar.gz

# The binary is now at: sing-box-1.11.7-linux-amd64/sing-box
```

**Alternative: Using the installation script**
```bash
curl -Lo /usr/local/bin/sing-box https://github.com/SagerNet/sing-box/releases/download/v1.11.7/sing-box-1.11.7-linux-amd64.tar.gz | tar -xzf - sing-box-1.11.7-linux-amd64/sing-box --strip-components=1
chmod +x /usr/local/bin/sing-box
```

### Step 2: Build or Download singboxer

**Option A: Build from source (requires Rust)**
```bash
git clone <your-repo-url>
cd singboxer
cargo build --release
# Binary is at: target/release/singboxer
```

**Option B: Download pre-built binary** (if you have one)
```bash
# Download and make executable
wget https://your-server/singboxer
chmod +x singboxer
```

### Step 3: Upload to Your Server

```bash
# Using scp
scp singbox sing-box-1.11.7-linux-amd64/sing-box user@your-server:/home/user/

# Or using rsync
scp target/release/singboxer user@your-server:/home/user/
```

### Step 4: Setup on Server

SSH into your server and run:

```bash
# Create directory for the binaries
mkdir -p ~/singbox
cd ~/singbox

# Move binaries here
mv ~/singbox ~/singbox/
mv ~/sing-box ~/singbox/  # or wherever you downloaded it

# Make them executable
chmod +x singbox singboxer

# Verify installation
./sing-box version
./singboxer --help
```

### Step 5: Add Your Subscription

```bash
# Add your subscription URL
./singboxer add "MyProvider" "https://your-subscription-url"
```

### Step 6: Start Using singboxer

```bash
# Launch the TUI
./singboxer

# Or run it in the background for remote access
nohup ./singboxer &
```

---

## Download Binaries

### sing-box

| Platform | Download Link |
|----------|---------------|
| Linux AMD64 | [sing-box-1.11.7-linux-amd64.tar.gz](https://github.com/SagerNet/sing-box/releases/download/v1.11.7/sing-box-1.11.7-linux-amd64.tar.gz) |
| Linux ARM64 | [sing-box-1.11.7-linux-arm64.tar.gz](https://github.com/SagerNet/sing-box/releases/download/v1.11.7/sing-box-1.11.7-linux-arm64.tar.gz) |
| macOS AMD64 | [sing-box-1.11.7-darwin-amd64.tar.gz](https://github.com/SagerNet/sing-box/releases/download/v1.11.7/sing-box-1.11.7-darwin-amd64.tar.gz) |
| macOS ARM64 | [sing-box-1.11.7-darwin-arm64.tar.gz](https://github.com/SagerNet/sing-box/releases/download/v1.11.7/sing-box-1.11.7-darwin-arm64.tar.gz) |

Check [latest releases](https://github.com/SagerNet/sing-box/releases) for the newest version.

### singboxer

If you don't have Rust installed, build locally and upload:

```bash
# On your local machine
cargo build --release
scp target/release/singboxer user@server:~/singbox/
```

---

## Upload to Server

### Method 1: Using scp

```bash
# From your local machine
scp target/release/singboxer user@your-server:~/singbox/
scp sing-box-1.11.7-linux-amd64/sing-box user@your-server:~/singbox/
```

### Method 2: Using rsync

```bash
rsync -avz target/release/singboxer user@your-server:~/singbox/
rsync -avz sing-box-1.11.7-linux-amd64/sing-box user@your-server:~/singbox/
```

### Method 3: Direct download on server

```bash
# SSH into your server first
ssh user@your-server

# Download sing-box
cd ~
mkdir -p singbox
cd singbox
wget https://github.com/SagerNet/sing-box/releases/download/v1.11.7/sing-box-1.11.7-linux-amd64.tar.gz
tar -xzf sing-box-1.11.7-linux-amd64.tar.gz
mv sing-box-1.11.7-linux-amd64/sing-box sing-box
chmod +x sing-box

# Download singboxer (or upload via scp)
wget https://your-cdn/singboxer
chmod +x singboxer
```

---

## Usage

### First Time Setup

```bash
# SSH to your server
ssh user@your-server

# Navigate to your singbox directory
cd ~/singbox

# Add your subscription
./singboxer add "MyProvider" "https://subscription-url.com"

# Launch the TUI
./singboxer
```

### TUI Key Bindings

| Key | Action |
|-----|--------|
| **Navigation** |
| `Tab` / `←` / `→` | Switch panels (Subscriptions / Proxies) |
| `↑` / `↓` | Navigate lists |
| **Actions** |
| `Enter` | Load subscription / Activate proxy |
| `S` | Start sing-box |
| `x` | Stop sing-box |
| `R` | Restart sing-box |
| `s` | Save config to file |
| `r` | Reload subscription |
| `t` | Test all proxy latencies |
| `T` | Test selected proxy |
| `d` | Delete selected subscription |
| `?` / `Esc` | Toggle help |
| `q` | Quit |

### Typical Workflow

1. **Launch singboxer**
   ```bash
   ./singboxer
   ```

2. **Load Proxies** - Select your subscription, press `Enter`

3. **Test Latencies** - Press `t` to test all proxies

4. **Select Fastest Proxy** - Navigate to the proxy with lowest latency

5. **Start sing-box** - Press `S` to start with selected proxy

6. **Use Your Proxy** - The TUN mode is now active, all traffic is proxied

7. **Switch Proxies** - Select a different proxy and press `Enter` (no restart!)

8. **Stop** - Press `x` to stop sing-box, or `q` to quit singboxer

---

## CLI Commands

You can also use CLI commands without the TUI:

```bash
# Add a subscription
./singboxer add "MyProvider" "https://subscription-url"

# List all subscriptions
./singboxer list

# Remove a subscription
./singboxer remove "MyProvider"

# Fetch and display proxies from a URL
./singboxer fetch "https://subscription-url"

# Generate config from a subscription URL
./singboxer generate "https://subscription-url" -o config.json

# Run the TUI
./singboxer
# or explicitly
./singboxer ui
```

---

## Configuration Files

All configurations are stored in `~/.config/singboxer/`:

```
~/.config/singboxer/
├── subscriptions.json    # Your subscription URLs
└── singbox/
    └── config.json       # Generated sing-box config
```

### Manual Configuration

You can also manually edit `subscriptions.json`:

```json
[
  {
    "name": "MyProvider",
    "url": "https://subscription-url",
    "type": "clash",
    "enabled": true
  }
]
```

---

## Running as a Service

To run sing-box as a system service (auto-start on boot):

### Create systemd service

```bash
sudo nano /etc/systemd/system/singbox.service
```

Add the following:

```ini
[Unit]
Description=sing-box Service
After=network.target

[Service]
Type=simple
User=your-username
WorkingDirectory=/home/your-username/singbox
ExecStart=/home/your-username/singbox/sing-box run -c /home/your-username/.config/singboxer/singbox/config.json
Restart=on-failure
RestartSec=5s

# For TUN mode, need CAP_NET_ADMIN
CapabilityBoundingSet=CAP_NET_ADMIN CAP_NET_RAW
AmbientCapabilities=CAP_NET_ADMIN CAP_NET_RAW

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl daemon-reload
sudo systemctl enable singbox
sudo systemctl start singbox
sudo systemctl status singbox
```

### Using singboxer for management only

If you want to manage the config but run sing-box separately:

```bash
# Use singboxer to generate/select configs
./singboxer

# Then run sing-box manually with the generated config
./sing-box run -c ~/.config/singboxer/singbox/config.json
```

---

## Troubleshooting

### sing-box not found

```
Error: sing-box not found
```

**Solution:** Install sing-box or make sure it's in your PATH:
```bash
# Add to PATH
export PATH=$PATH:~/singbox

# Or create a symlink
sudo ln -s ~/singbox/sing-box /usr/local/bin/sing-box
```

### Permission denied with TUN mode

```
Error: failed to initialize tun
```

**Solution:** Run with sudo or add capabilities:
```bash
# Run with sudo
sudo ./sing-box run -c config.json

# Or add capabilities (recommended)
sudo setcap cap_net_admin,cap_net_raw+ep /path/to/sing-box
```

### Can't connect to Clash API

```
Error: Failed to connect to Clash API
```

**Solution:** Make sure sing-box is running and Clash API is enabled in the config.

---

## Features

### Supported Subscription Formats

- **Clash YAML** - Most common format
- **Shadowsocks/SIP008** - URL list format
- **V2RayN URIs** - vmess://, vless://, trojan://, ss://
- **Base64 encoded** - Auto-detection

### Supported Proxy Types

- Shadowsocks
- VMess
- VLESS (with Reality)
- Trojan
- Hysteria2
- SOCKS5
- HTTP/HTTPS

### Generated Config Features

- **TUN inbound** - Transparent proxy with auto-routing
- **SOCKS5 inbound** - Local proxy on `127.0.0.1:7890`
- **HTTP inbound** - Local proxy on `127.0.0.1:7891`
- **Selector outbound** - Manual proxy selection
- **URLTest outbound** - Auto-select fastest proxy
- **Clash API** - Control via dashboards (Yacd, MetaCubeXD)
- **Routing rules** - Direct for private/CN IPs/domains

### Latency Testing

- Color-coded results (green/yellow/orange/red)
- Concurrent testing (up to 5 at once)
- TCP connection test for protocol proxies
- Full HTTP test for SOCKS/HTTP proxies

---

## License

MIT
