#!/usr/bin/env bash
# install.sh - Install singboxer shell scripts

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Config directories
CONFIG_DIR="$HOME/.config/singboxer"
BIN_DIR="$CONFIG_DIR/bin"

# Detect shell rc file
detect_rc() {
    if [ -n "$ZSH_VERSION" ]; then
        echo "$HOME/.zshrc"
    elif [ -n "$BASH_VERSION" ]; then
        echo "$HOME/.bashrc"
    else
        echo "$HOME/.profile"
    fi
}

# Main installation
main() {
    echo ""
    echo -e "${CYAN}=== singboxer Shell Script Installation ===${NC}"
    echo ""

    # Create directories
    log_info "Creating directories..."
    mkdir -p "$BIN_DIR"
    mkdir -p "$HOME/.config/sing-box"

    # Get script directory (where install.sh is located)
    local script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

    # Look for scripts in bin/ subdirectory or current directory
    local src_dir=""
    if [ -d "$script_dir/bin" ] && [ -f "$script_dir/bin/singboxer-start" ]; then
        src_dir="$script_dir/bin"
    elif [ -f "$script_dir/singboxer-start" ]; then
        src_dir="$script_dir"
    else
        log_error "Cannot find singboxer scripts"
        exit 1
    fi

    log_info "Installing scripts from $src_dir..."

    # Copy and make scripts executable
    for script in singboxer-start singboxer-stop singboxer-status singboxer-list \
                  singboxer-select singboxer-env singboxer-check; do
        local src="$src_dir/$script"
        local dest="$BIN_DIR/$script"
        if [ -f "$src" ]; then
            cp "$src" "$dest"
            chmod +x "$dest"
            log_info "  + $script"
        else
            log_error "  - $script not found"
        fi
    done

    # Check for dependencies
    echo ""
    log_info "Checking dependencies..."

    if command -v sing-box &> /dev/null; then
        local version=$(sing-box version 2>/dev/null || echo "unknown")
        echo -e "  sing-box: ${GREEN}installed${NC} ($version)"
    else
        echo -e "  sing-box: ${RED}not found${NC}"
        echo ""
        log_error "sing-box is required!"
        echo ""
        echo "Install from: https://github.com/SagerNet/sing-box/releases"
        echo ""
        echo "Or on macOS with Homebrew:"
        echo "  brew install sing-box"
        echo ""
        echo "Or on Linux:"
        echo "  curl -fsSL https://github.com/SagerNet/sing-box/releases/download/v1.12.3/sing-box-1.12.3-linux-amd64.tar.gz | tar xz"
        echo "  sudo mv sing-box-1.12.3-linux-amd64/sing-box /usr/local/bin/"
    fi

    if command -v jq &> /dev/null; then
        echo -e "  jq: ${GREEN}installed${NC}"
    else
        echo -e "  jq: ${YELLOW}not found (recommended)${NC}"
        echo ""
        echo "Install with:"
        echo "  macOS: brew install jq"
        echo "  Linux: sudo apt install jq  # or dnf install jq, etc."
    fi

    if command -v yq &> /dev/null; then
        echo -e "  yq: ${GREEN}installed${NC}"
    else
        echo -e "  yq: ${YELLOW}not found (optional, for YAML parsing)${NC}"
    fi

    if command -v curl &> /dev/null; then
        echo -e "  curl: ${GREEN}installed${NC}"
    else
        echo -e "  curl: ${RED}not found${NC} (required)"
    fi

    # Update shell rc
    echo ""
    local rc_file=$(detect_rc)
    local export_line="export PATH=\"$BIN_DIR:\$PATH\""

    if grep -q "$BIN_DIR" "$rc_file" 2>/dev/null; then
        log_info "PATH already configured in $rc_file"
    else
        echo ""
        echo -e "${CYAN}Add to PATH?${NC} This will add the following line to $rc_file:"
        echo "  $export_line"
        echo ""
        read -p "Add to PATH? [Y/n] " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Nn]$ ]]; then
            echo "" >> "$rc_file"
            echo "# singboxer" >> "$rc_file"
            echo "$export_line" >> "$rc_file"
            log_info "Added to $rc_file"
            log_info "Run 'source $rc_file' or restart your shell"
        fi
    fi

    # Create sample subscription file
    if [ ! -f "$CONFIG_DIR/subscriptions.conf" ]; then
        echo ""
        log_info "Creating sample subscriptions.conf..."
        cat > "$CONFIG_DIR/subscriptions.conf" << 'EOF'
# singboxer subscription configuration
# Add your subscription URLs below (one per line)
# Lines starting with # are ignored

# Example subscription URLs:
# https://your-subscription-provider.com/link/xyz
# file:///path/to/local/clash/config.yaml
EOF
        log_info "Created $CONFIG_DIR/subscriptions.conf"
        log_info "Edit this file and add your subscription URL"
    fi

    # Done
    echo ""
    echo -e "${GREEN}=== Installation Complete ===${NC}"
    echo ""
    echo "Commands available:"
    echo "  singboxer-start   - Fetch subscription and start sing-box"
    echo "  singboxer-stop    - Stop sing-box"
    echo "  singboxer-status  - Show status"
    echo "  singboxer-list    - List available proxies"
    echo "  singboxer-select  - Select a proxy"
    echo "  singboxer-check   - Test connectivity"
    echo "  singboxer-env     - Show proxy env vars"
    echo ""
    echo "Quick start:"
    echo "  1. Edit $CONFIG_DIR/subscriptions.conf and add your subscription URL"
    echo "  2. Run: singboxer-start"
    echo "  3. Set proxy: eval \$(singboxer-env)"
    echo "  4. Check: singboxer-check"
    echo ""
}

main "$@"
