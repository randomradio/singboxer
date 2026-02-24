# Troubleshooting

## Build Issues

### OpenSSL not found (Ubuntu/ARM)

**Error:**
```
Could not find directory of OpenSSL installation
```

**Solution: Install OpenSSL development headers**

```bash
# Ubuntu/Debian
sudo apt-get update
sudo apt-get install -y pkg-config libssl-dev

# Ubuntu ARM64 specific
sudo apt-get install -y libssl-dev:arm64
```

### Alternative: Use vendored OpenSSL

If you cannot install system OpenSSL, use the `vendored` feature:

```bash
# Edit Cargo.toml
sed -i 's/openssl-sys/openssl/g' Cargo.toml

# Build with vendored OpenSSL
cargo build --release
```

This compiles OpenSSL from source (takes longer but no system dependencies).

### Alternative: Static binary on host machine

Build on a machine with proper toolchain:

```bash
# On x86_64 machine with proper Rust toolchain
cargo build --release --target aarch64-unknown-linux-gnu

# Cross-compile for ARM64
rustup target add aarch64-unknown-linux-gnu
cargo build --release --target aarch64-unknown-linux-gnu
```

Then upload the binary to your ARM server.

---

## Runtime Issues

### sing-box: Not Installed

**Error:** `sing-box: Not Installed` in header

**Solutions:**

```bash
# Check if sing-box is in PATH
which sing-box

# Add to PATH if not
export PATH=$PATH:~/singbox

# Or create symlink
sudo ln -s ~/singbox/sing-box /usr/local/bin/sing-box
```

### Permission denied with TUN mode

**Error:** `failed to initialize tun`

**Solutions:**

```bash
# Option 1: Run with sudo
sudo ./sing-box run -c config.json

# Option 2: Add capabilities (recommended)
sudo setcap cap_net_admin,cap_net_raw+ep ~/singbox/sing-box
```

### Can't connect to Clash API

**Error:** Proxy switching fails

**Solutions:**

1. Make sure sing-box is running
2. Check Clash API is enabled in generated config
3. Verify API port (default: 9090) is not blocked by firewall

```bash
# Check if API is accessible
curl http://127.0.0.1:9090/proxies

# Check firewall
sudo ufw status
sudo ufw allow 9090/tcp
```

### Subscription not loading

**Check:**

```bash
# Test the URL directly
curl -L "your-subscription-url"

# For files, check path
ls -la /path/to/file

# Enable debug logging
RUST_LOG=debug ./singboxer
```

---

## Tips for Ubuntu/ARM Servers

### Install build dependencies

```bash
sudo apt-get update
sudo apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    clang \
    curl
```

### For cross-compilation from x86_64

```bash
# Install ARM toolchain
sudo apt-get install gcc-aarch64-linux-gnu

# Add Rust target
rustup target add aarch64-unknown-linux-gnu

# Build for ARM
cargo build --release --target aarch64-unknown-linux-gnu
```
