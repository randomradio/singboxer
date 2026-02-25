# singboxer - Shell Script Edition

Simple shell scripts to manage sing-box proxy configurations. No Rust compilation required.

## Installation

```bash
cd ~/.config/singboxer
./install.sh
```

Or manually add to PATH:

```bash
export PATH="$HOME/.config/singboxer/bin:$PATH"
```

## Requirements

- `sing-box` - The core proxy binary
- `jq` - For JSON processing
- `curl` - For fetching subscriptions
- `python3` - For config generation (fallback YAML parsing)

Optional:
- `yq` - For better YAML parsing

## Configuration

Edit `~/.config/singboxer/subscriptions.conf`:

```bash
# Add your subscription URL (one per line)
https://your-subscription-provider.com/link/xyz
```

## Usage

### Start sing-box

```bash
singboxer-start
```

This will:
1. Fetch your subscription
2. Parse the proxy list
3. Generate a sing-box config
4. Start sing-box in the background

### Stop sing-box

```bash
singboxer-stop
```

### Check status

```bash
singboxer-status
```

### List available proxies

```bash
singboxer-list
```

### Select a proxy

```bash
# By number
singboxer-select 1

# By name (fuzzy match)
singboxer-select hk
singboxer-select us-01
```

### Set proxy environment variables

```bash
eval $(singboxer-env)
```

Or add to your shell:

```bash
export http_proxy=http://127.0.0.1:7891
export https_proxy=http://127.0.0.1:7891
export all_proxy=socks5://127.0.0.1:7890
```

### Test connectivity

```bash
singboxer-check
```

## Directory Structure

```
~/.config/singboxer/
├── bin/
│   ├── singboxer-start   # Start sing-box
│   ├── singboxer-stop    # Stop sing-box
│   ├── singboxer-status  # Show status
│   ├── singboxer-list    # List proxies
│   ├── singboxer-select  # Select proxy
│   ├── singboxer-env     # Print env exports
│   └── singboxer-check   # Test connectivity
├── subscriptions.conf    # Your subscription URLs
├── proxy_cache.json      # Cached proxy list
└── install.sh            # Installation script

~/.config/sing-box/
└── config.json           # Generated sing-box config
```

## Environment Variables

- `SINGBOXER_SOCKS_PORT` - SOCKS5 port (default: 7890)
- `SINGBOXER_HTTP_PORT` - HTTP proxy port (default: 7891)
- `SINGBOXER_API_PORT` - Clash API port (default: 9090)
- `CONFIG_DIR` - Config directory (default: ~/.config/singboxer)

## Supported Subscription Formats

- Clash YAML (with yq or python3)
- VMess URIs (vmess://)
- VLESS URIs (vless://)
- Trojan URIs (trojan://)
- Shadowsocks URIs (ss://)

## License

MIT
