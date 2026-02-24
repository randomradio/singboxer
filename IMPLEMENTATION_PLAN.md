# Implementation Plan: singboxer TUI

## Overview
A terminal UI app for managing sing-box configurations. Users can import Clash/Shadowsocks subscription links, view proxies, select nodes, and manage configuration.

## Tech Stack
- **Language**: Rust
- **TUI Framework**: ratatui (formerly tui-rs)
- **HTTP Client**: reqwest
- **Async Runtime**: tokio
- **Config Parsing**: serde_json (for sing-box JSON), serde_yaml (for Clash YAML)

---

## Stage 1: Project Foundation
**Goal**: Basic Rust project with dependencies and core data structures

**Tasks**:
- Initialize Cargo project
- Add dependencies (ratatui, reqwest, tokio, serde, crossterm)
- Define core data models:
  - `Subscription` (url, type, name)
  - `ProxyServer` (name, type, latency, country)
  - `SingBoxConfig` (full config structure)
- Create basic config file storage (`~/.config/singboxer/`)

**Success Criteria**:
- `cargo build` succeeds
- Data models compile with serde derives

**Status**: Complete

---

## Stage 2: Subscription Fetching & Parsing
**Goal**: Fetch and parse subscription URLs

**Tasks**:
- Implement `fetch_subscription(url)` function
- Parse Clash YAML format
- Parse Shadowsocks/SIP008 format
- Parse V2RayN URI format (vmess://, vless://, trojan://)
- Convert parsed proxies to sing-box outbound format
- Handle base64 encoded subscription content
- Error handling for network/fetch failures

**Success Criteria**:
- Can fetch a Clash subscription URL
- Parsed proxies display with names
- Converts to sing-box JSON structure

**Tests**:
- Mock HTTP response for Clash YAML
- Mock HTTP response for Shadowsocks URL list
- Verify conversion to sing-box outbounds

**Status**: Complete

---

## Stage 3: TUI Framework
**Goal**: Basic TUI with navigation

**Tasks**:
- Setup ratatui terminal
- Create main layout:
  - Left panel: Subscription list
  - Right panel: Proxy list for selected sub
- Keyboard navigation (arrow keys, enter, esc)
- State management (current sub, current proxy)

**Success Criteria**:
- TUI renders without crash
- Can navigate between panels
- Can select subscriptions

**Status**: Complete

---

## Stage 4: Proxy Selection & Config Generation
**Goal**: Select proxies and generate sing-box config

**Tasks**:
- Implement selector group creation
- Allow user to pick active proxy
- Generate complete sing-box config.json
- Write config to file (or output to stdout)
- Support saving profiles

**Success Criteria**:
- Can select a proxy from list
- Generated config is valid sing-box JSON
- Config can be written to file

**Tests**:
- Verify generated JSON matches sing-box schema
- Test with selector, urltest outbounds

**Status**: Complete

---

## Stage 5: Polish & Extras
**Goal**: CLI UX improvements

**Tasks**:
- [ ] Latency testing for proxies
- [ ] Search/filter proxies
- [ ] Color-coded countries/flags
- [ ] Health check / URL test integration
- [ ] Input dialog for adding subscriptions in TUI

**Success Criteria**:
- Smooth user experience
- Help text and key bindings displayed

**Status**: In Progress
