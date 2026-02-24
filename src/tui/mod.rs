// Terminal UI for singboxer

use crate::config::AppConfig;
use crate::latency::{test_proxies_concurrent, format_latency};
use crate::models::{Subscription, ProxyServer};
use crate::parser::{fetch_subscription, parse_clash_yaml, parse_shadowsocks_urls, parse_v2ray_uri, is_base64, decode_base64};
use crate::singbox::{SingBoxManager, SingBoxStatus, SINGBOX_GITHUB};
use crate::parser::sanitize_tag;
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;

/// App state
pub struct App {
    pub config: AppConfig,
    pub subscriptions: Vec<Subscription>,
    pub proxies: Vec<ProxyServer>,
    pub selected_sub: usize,
    pub selected_proxy: usize,
    pub status: String,
    pub loading: bool,
    pub current_panel: Panel,
    pub singbox: SingBoxManager,
    pub singbox_status: SingBoxStatus,
    pub testing_proxy: Option<usize>,
    pub show_help: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Subscriptions,
    Proxies,
    Actions,
}

impl App {
    pub fn new() -> Result<Self> {
        let config = AppConfig::default();
        config.init()?;

        let subscriptions = config.load_subscriptions().unwrap_or_default();
        let singbox = SingBoxManager::new();
        let singbox_status = singbox.status();

        Ok(Self {
            config,
            subscriptions,
            proxies: Vec::new(),
            selected_sub: 0,
            selected_proxy: 0,
            status: "Press 'a' to add subscription, 'q' to quit".to_string(),
            loading: false,
            current_panel: Panel::Subscriptions,
            singbox,
            singbox_status,
            testing_proxy: None,
            show_help: false,
        })
    }

    /// Update sing-box status
    pub fn update_singbox_status(&mut self) {
        self.singbox_status = self.singbox.status();
    }

    /// Check if sing-box is installed
    pub fn check_singbox(&mut self) -> bool {
        match self.singbox.check_installation() {
            Ok(_) => {
                self.update_singbox_status();
                true
            }
            Err(_) => {
                self.singbox_status = SingBoxStatus::NotFound;
                false
            }
        }
    }

    /// Get sing-box version
    pub fn get_singbox_version(&self) -> Option<String> {
        if self.singbox_status == SingBoxStatus::NotFound {
            return None;
        }
        self.singbox.get_version().ok()
    }

    /// Start sing-box with current configuration
    pub fn start_singbox(&mut self, rt: &Runtime) -> Result<()> {
        if self.singbox_status == SingBoxStatus::NotFound {
            return Err(anyhow::anyhow!("sing-box not found. Install from {}", SINGBOX_GITHUB));
        }

        // Generate config first
        let config_path = self.config.singbox_config_dir.join("config.json");
        let selected = self.proxies.get(self.selected_proxy).map(|p| p.name.as_str());
        let config = crate::config::generate_singbox_config(&self.proxies, selected)?;
        crate::config::save_singbox_config(&config, &config_path)?;

        match self.singbox.start(&config_path) {
            Ok(pid) => {
                self.status = format!("sing-box started (PID: {})", pid);
                self.update_singbox_status();
                Ok(())
            }
            Err(e) => {
                self.status = format!("Failed to start sing-box: {}", e);
                Err(e)
            }
        }
    }

    /// Stop sing-box
    pub fn stop_singbox(&mut self) -> Result<()> {
        match self.singbox.stop() {
            Ok(_) => {
                self.status = "sing-box stopped".to_string();
                self.update_singbox_status();
                Ok(())
            }
            Err(e) => {
                self.status = format!("Failed to stop sing-box: {}", e);
                Err(e)
            }
        }
    }

    /// Restart sing-box
    pub fn restart_singbox(&mut self, rt: &Runtime) -> Result<()> {
        self.stop_singbox()?;
        std::thread::sleep(std::time::Duration::from_millis(500));
        self.start_singbox(rt)
    }

    /// Switch to selected proxy via Clash API
    pub async fn switch_proxy(&mut self, rt: &Runtime) -> Result<()> {
        if self.singbox_status != SingBoxStatus::Available {
            // Check if it's actually running
            self.update_singbox_status();
        }

        let proxy = self.proxies.get(self.selected_proxy)
            .ok_or_else(|| anyhow::anyhow!("No proxy selected"))?;

        let tag = sanitize_tag(&proxy.name);
        let config_path = self.config.singbox_config_dir.join("config.json");

        match self.singbox.switch_proxy("proxy", &tag, &config_path).await {
            Ok(_) => {
                self.status = format!("Switched to: {}", proxy.name);
                Ok(())
            }
            Err(e) => {
                self.status = format!("Failed to switch: {}. Start sing-box first?", e);
                Err(e)
            }
        }
    }

    /// Test latency for all proxies
    pub async fn test_all_latencies(&mut self, _rt: &Runtime) {
        if self.proxies.is_empty() {
            self.status = "No proxies to test".to_string();
            return;
        }

        self.status = "Testing latencies...".to_string();

        // Test up to 5 proxies concurrently
        let results = test_proxies_concurrent(&mut self.proxies, None, 5).await;

        // Process results
        let mut success_count = 0;
        let mut total_latency = 0u64;

        for (i, result) in &results {
            match result {
                Ok(latency) => {
                    self.proxies[*i].latency_ms = Some(latency.as_millis() as u64);
                    success_count += 1;
                    total_latency += latency.as_millis() as u64;
                }
                Err(_) => {
                    self.proxies[*i].latency_ms = None;
                }
            }
        }

        let avg_latency = if success_count > 0 {
            total_latency / success_count as u64
        } else {
            0
        };

        self.status = format!(
            "Tested {} proxies | {} responsive | Avg: {}ms",
            self.proxies.len(),
            success_count,
            avg_latency
        );
    }

    /// Test latency for a single proxy
    pub async fn test_single_proxy(&mut self, index: usize, _rt: &Runtime) {
        if index >= self.proxies.len() {
            return;
        }

        self.testing_proxy = Some(index);
        self.status = format!("Testing {}...", self.proxies[index].name);

        match crate::latency::test_proxy_latency(&self.proxies[index], None).await {
            Ok(latency) => {
                self.proxies[index].latency_ms = Some(latency.as_millis() as u64);
                self.status = format!(
                    "{}: {}ms",
                    self.proxies[index].name,
                    latency.as_millis()
                );
            }
            Err(_) => {
                self.proxies[index].latency_ms = None;
                self.status = format!(
                    "{}: Timeout/Error",
                    self.proxies[index].name
                );
            }
        }

        self.testing_proxy = None;
    }

    pub fn add_subscription(&mut self, name: String, url: String) {
        let sub_type = Self::detect_subscription_type(&url);
        let name_clone = name.clone();
        let sub = Subscription::new(name, url, sub_type);
        self.subscriptions.push(sub);
        let _ = self.config.save_subscriptions(&self.subscriptions);
        self.status = format!("Added subscription: {}", name_clone);
    }

    pub fn remove_subscription(&mut self) {
        if self.subscriptions.is_empty() {
            self.status = "No subscriptions to remove".to_string();
            return;
        }

        let name = self.subscriptions[self.selected_sub].name.clone();
        self.subscriptions.remove(self.selected_sub);
        if self.selected_sub >= self.subscriptions.len() && !self.subscriptions.is_empty() {
            self.selected_sub = self.subscriptions.len() - 1;
        }
        let _ = self.config.save_subscriptions(&self.subscriptions);
        self.status = format!("Removed subscription: {}", name);
    }

    pub async fn load_proxies(&mut self, rt: &Runtime) -> Result<()> {
        if self.subscriptions.is_empty() {
            self.status = "No subscriptions to load".to_string();
            return Ok(());
        }

        self.loading = true;
        self.status = "Loading proxies...".to_string();

        let sub = self.subscriptions[self.selected_sub].clone();

        match rt.spawn(async move {
            let content = fetch_subscription(&sub.url).await?;
            let proxies = Self::parse_subscription_content(&content, &sub)?;
            Ok::<_, anyhow::Error>(proxies)
        }).await {
            Ok(Ok(proxies)) => {
                self.proxies = proxies;
                self.selected_proxy = 0;
                self.status = format!("Loaded {} proxies", self.proxies.len());
            }
            Ok(Err(e)) => {
                self.status = format!("Error loading: {}", e);
            }
            Err(e) => {
                self.status = format!("Task error: {}", e);
            }
        }

        self.loading = false;
        Ok(())
    }

    pub fn detect_subscription_type(url: &str) -> crate::models::SubscriptionType {
        if url.contains("/clash") || url.contains("clash") {
            crate::models::SubscriptionType::Clash
        } else if url.contains("/shadowsocks") || url.contains("/ss") {
            crate::models::SubscriptionType::Shadowsocks
        } else if url.contains("/v2ray") || url.contains("/vmess") {
            crate::models::SubscriptionType::V2Ray
        } else if url.contains("/singbox") || url.contains("/sing-box") {
            crate::models::SubscriptionType::Singbox
        } else {
            crate::models::SubscriptionType::Auto
        }
    }

    pub fn parse_subscription_content(content: &str, _sub: &Subscription) -> Result<Vec<ProxyServer>> {
        // Try different formats
        let trimmed = content.trim();

        // Check if it's base64 encoded
        let decoded = if is_base64(trimmed) {
            decode_base64(trimmed).unwrap_or_else(|_| trimmed.to_string())
        } else {
            trimmed.to_string()
        };

        // Try Clash YAML first
        if let Ok(proxies) = parse_clash_yaml(&decoded) {
            if !proxies.is_empty() {
                return Ok(proxies);
            }
        }

        // Try as newline-separated URLs (v2rayn format)
        if decoded.lines().any(|l| l.starts_with("vmess://") || l.starts_with("vless://") || l.starts_with("trojan://")) {
            let mut proxies = Vec::new();
            for line in decoded.lines() {
                if let Ok(proxy) = parse_v2ray_uri(line.trim()) {
                    proxies.push(proxy);
                }
            }
            if !proxies.is_empty() {
                return Ok(proxies);
            }
        }

        // Try Shadowsocks URL list
        if let Ok(proxies) = parse_shadowsocks_urls(&decoded) {
            if !proxies.is_empty() {
                return Ok(proxies);
            }
        }

        Err(anyhow::anyhow!("Could not parse subscription"))
    }

    pub fn generate_config(&self) -> Result<String> {
        if self.proxies.is_empty() {
            return Ok("No proxies loaded".to_string());
        }

        let selected = self.proxies.get(self.selected_proxy).map(|p| p.name.as_str());
        let config = crate::config::generate_singbox_config(&self.proxies, selected)?;
        let config_path = self.config.singbox_config_dir.join("config.json");
        crate::config::save_singbox_config(&config, &config_path)?;
        Ok(format!("Config saved to: {}", config_path.display()))
    }

    pub fn next_sub(&mut self) {
        if !self.subscriptions.is_empty() {
            self.selected_sub = (self.selected_sub + 1) % self.subscriptions.len();
        }
    }

    pub fn prev_sub(&mut self) {
        if !self.subscriptions.is_empty() {
            self.selected_sub = if self.selected_sub == 0 {
                self.subscriptions.len() - 1
            } else {
                self.selected_sub - 1
            };
        }
    }

    pub fn next_proxy(&mut self) {
        if !self.proxies.is_empty() {
            self.selected_proxy = (self.selected_proxy + 1) % self.proxies.len();
        }
    }

    pub fn prev_proxy(&mut self) {
        if !self.proxies.is_empty() {
            self.selected_proxy = if self.selected_proxy == 0 {
                self.proxies.len() - 1
            } else {
                self.selected_proxy - 1
            };
        }
    }

    pub fn switch_panel(&mut self) {
        self.current_panel = match self.current_panel {
            Panel::Subscriptions => Panel::Proxies,
            Panel::Proxies => Panel::Actions,
            Panel::Actions => Panel::Subscriptions,
        };
    }
}

/// Run the TUI
pub fn run_ui(app: App) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let rt = Runtime::new()?;
    let app = Arc::new(Mutex::new(app));

    let result = run_app(&mut terminal, Arc::clone(&app), &rt);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    app: Arc<Mutex<App>>,
    rt: &Runtime,
) -> Result<()> {
    loop {
        let mut app_guard = app.lock().unwrap();
        terminal.draw(|f| ui(f, &mut app_guard))?;

        if let Event::Key(key) = event::read()? {
            let app_clone = Arc::clone(&app);
            let rt_clone = rt.handle().clone();

            match key.code {
                KeyCode::Char('q') => {
                    return Ok(());
                }
                KeyCode::Char('?') => {
                    app_guard.show_help = !app_guard.show_help;
                }
                KeyCode::Esc => {
                    app_guard.show_help = false;
                }
                KeyCode::Tab | KeyCode::BackTab => {
                    app_guard.switch_panel();
                }
                KeyCode::Left => {
                    if app_guard.current_panel == Panel::Proxies {
                        app_guard.current_panel = Panel::Subscriptions;
                    }
                }
                KeyCode::Right => {
                    if app_guard.current_panel == Panel::Subscriptions {
                        app_guard.current_panel = Panel::Proxies;
                    }
                }
                KeyCode::Down => {
                    match app_guard.current_panel {
                        Panel::Subscriptions => app_guard.next_sub(),
                        Panel::Proxies => app_guard.next_proxy(),
                        _ => {}
                    }
                }
                KeyCode::Up => {
                    match app_guard.current_panel {
                        Panel::Subscriptions => app_guard.prev_sub(),
                        Panel::Proxies => app_guard.prev_proxy(),
                        _ => {}
                    }
                }
                KeyCode::Enter => {
                    if app_guard.current_panel == Panel::Subscriptions {
                        let _selected = app_guard.selected_sub;
                        drop(app_guard);
                        let mut app = app_clone.lock().unwrap();
                        rt_clone.block_on(app.load_proxies(rt))?;
                    } else if app_guard.current_panel == Panel::Proxies && !app_guard.proxies.is_empty() {
                        // Activate selected proxy via Clash API
                        let _selected = app_guard.selected_proxy;
                        drop(app_guard);
                        let mut app = app_clone.lock().unwrap();
                        let _ = rt_clone.block_on(app.switch_proxy(rt));
                    }
                }
                KeyCode::Char('a') => {
                    // Add subscription (simplified - would normally show input dialog)
                    drop(app_guard);
                    // TODO: Implement input dialog
                    let mut app = app_clone.lock().unwrap();
                    app.status = "Add subscription via CLI: singboxer add <name> <url>".to_string();
                }
                KeyCode::Char('d') => {
                    app_guard.remove_subscription();
                }
                KeyCode::Char('s') => {
                    // Save config only (lowercase s)
                    let result = app_guard.generate_config();
                    match result {
                        Ok(msg) => app_guard.status = msg,
                        Err(e) => app_guard.status = format!("Error: {}", e),
                    }
                }
                KeyCode::Char('S') => {
                    // Start sing-box (uppercase S)
                    drop(app_guard);
                    let mut app = app_clone.lock().unwrap();
                    if !app.check_singbox() {
                        app.status = format!("sing-box not found. Install from {}", SINGBOX_GITHUB);
                    } else {
                        let _ = app.start_singbox(rt);
                    }
                }
                KeyCode::Char('x') => {
                    // Stop sing-box
                    drop(app_guard);
                    let mut app = app_clone.lock().unwrap();
                    let _ = app.stop_singbox();
                }
                KeyCode::Char('R') => {
                    // Restart sing-box (uppercase R)
                    drop(app_guard);
                    let mut app = app_clone.lock().unwrap();
                    if !app.check_singbox() {
                        app.status = format!("sing-box not found. Install from {}", SINGBOX_GITHUB);
                    } else {
                        let _ = app.restart_singbox(rt);
                    }
                }
                KeyCode::Char('r') => {
                    // Reload subscription (lowercase r)
                    let _selected = app_guard.selected_sub;
                    drop(app_guard);
                    let mut app = app_clone.lock().unwrap();
                    rt.block_on(app.load_proxies(rt))?;
                }
                KeyCode::Char('t') => {
                    // Test all latencies
                    drop(app_guard);
                    let mut app = app_clone.lock().unwrap();
                    rt_clone.block_on(app.test_all_latencies(rt));
                }
                KeyCode::Char('T') => {
                    // Test single proxy (uppercase T)
                    if app_guard.current_panel == Panel::Proxies && !app_guard.proxies.is_empty() {
                        let idx = app_guard.selected_proxy;
                        drop(app_guard);
                        let mut app = app_clone.lock().unwrap();
                        rt_clone.block_on(app.test_single_proxy(idx, rt));
                    }
                }
                KeyCode::Char('l') => {
                    app_guard.current_panel = Panel::Subscriptions;
                }
                KeyCode::Char('p') => {
                    app_guard.current_panel = Panel::Proxies;
                }
                _ => {}
            }
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let size = f.area();

    // Main layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(size);

    // Header with sing-box status
    let singbox_status_text = match app.singbox_status {
        SingBoxStatus::NotFound => Span::styled("Not Installed", Style::default().fg(Color::Red)),
        SingBoxStatus::Available => Span::styled("Stopped", Style::default().fg(Color::Yellow)),
        SingBoxStatus::Running { pid } => {
            Span::styled(format!("Running (PID: {})", pid), Style::default().fg(Color::Green))
        }
        SingBoxStatus::Stopped => Span::styled("Stopped", Style::default().fg(Color::Yellow)),
    };

    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("singboxer", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw(" - sing-box: "),
            singbox_status_text,
        ])
    ])
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL));

    f.render_widget(header, chunks[0]);

    // Content area
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(chunks[1]);

    // Subscriptions panel
    let sub_items: Vec<ListItem> = app
        .subscriptions
        .iter()
        .enumerate()
        .map(|(i, sub)| {
            let style = if i == app.selected_sub && app.current_panel == Panel::Subscriptions {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };
            ListItem::new(format!("{} | {:?}", sub.name, sub.sub_type)).style(style)
        })
        .collect();

    let sub_list = List::new(sub_items)
        .block(
            Block::default()
                .title("Subscriptions [l]")
                .borders(Borders::ALL)
                .border_style(if app.current_panel == Panel::Subscriptions {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default()
                })
        )
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));

    f.render_widget(sub_list, content_chunks[0]);

    // Proxies panel
    let proxy_items: Vec<ListItem> = app
        .proxies
        .iter()
        .enumerate()
        .map(|(i, proxy)| {
            let is_selected = i == app.selected_proxy && app.current_panel == Panel::Proxies;
            let is_testing = app.testing_proxy == Some(i);

            let style = if is_selected {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };

            let latency = if is_testing {
                format_latency(None, true)
            } else {
                format_latency(proxy.latency_ms, false)
            };

            let country = proxy.country.as_deref().unwrap_or("??");
            let proxy_type = format_proxy_type(&proxy.proxy_type);

            // Color code the latency
            let latency_span = if is_testing {
                Span::styled(
                    latency.clone(),
                    Style::default().fg(Color::Yellow)
                )
            } else {
                match proxy.latency_ms {
                    Some(ms) => {
                        let color = match ms {
                            0..=99 => Color::Green,
                            100..=299 => Color::Yellow,
                            300..=999 => Color::Rgb(255, 165, 0), // Orange
                            _ => Color::Red,
                        };
                        Span::styled(latency, Style::default().fg(color))
                    }
                    None => Span::styled(latency, Style::default().fg(Color::DarkGray)),
                }
            };

            let text = Line::from(vec![
                Span::styled(format!("{} ", country), Style::default().fg(Color::Cyan)),
                Span::raw("| "),
                Span::styled(format!("{} ", proxy.name), style),
                Span::raw("| "),
                Span::styled(format!("{} ", proxy_type), Style::default().fg(Color::Blue)),
                Span::raw("| "),
                latency_span,
            ]);

            ListItem::new(text).style(style)
        })
        .collect();

    let proxy_list = List::new(proxy_items)
        .block(
            Block::default()
                .title("Proxies [p]")
                .borders(Borders::ALL)
                .border_style(if app.current_panel == Panel::Proxies {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default()
                })
        );

    f.render_widget(proxy_list, content_chunks[1]);

    // Status bar with hints
    let hints = match app.singbox_status {
        SingBoxStatus::NotFound => "a:add d:del r:reload T:test t:all q:quit ?:help".to_string(),
        SingBoxStatus::Available => "a:add s:save S:start r:reload T:test t:all q:quit ?:help".to_string(),
        SingBoxStatus::Running { .. } => "a:add x:stop Enter:activate r:reload T:test t:all q:quit ?:help".to_string(),
        SingBoxStatus::Stopped => "a:add s:save S:start r:reload T:test t:all q:quit ?:help".to_string(),
    };

    let status_text = if app.status.contains("sing-box not found") {
        app.status.clone()
    } else {
        format!("{} | {}", app.status, hints)
    };

    let status = Paragraph::new(status_text.as_str())
        .block(Block::default().borders(Borders::ALL))
        .wrap(Wrap { trim: true });

    f.render_widget(status, chunks[2]);

    // Help popup
    if app.show_help {
        let help_text = vec![
            Line::from(vec![
                Span::styled("Key Bindings", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from("Navigation:"),
            Line::from("  Tab/←/→  - Switch panels"),
            Line::from("  ↑/↓       - Navigate lists"),
            Line::from(""),
            Line::from("Actions:"),
            Line::from("  Enter      - Load subscription / Activate proxy"),
            Line::from("  S          - Start sing-box"),
            Line::from("  x          - Stop sing-box"),
            Line::from("  R          - Restart sing-box"),
            Line::from("  s          - Save config to file"),
            Line::from("  r          - Reload subscription"),
            Line::from("  t          - Test all proxy latencies"),
            Line::from("  T          - Test selected proxy latency"),
            Line::from("  a          - Add subscription (via CLI)"),
            Line::from("  d          - Delete subscription"),
            Line::from("  q          - Quit"),
            Line::from("  Esc/?      - Toggle this help"),
            Line::from(""),
            Line::from("Latency Colors:"),
            Line::from("  <100ms     - Green (fast)"),
            Line::from("  100-300ms  - Yellow (good)"),
            Line::from("  300-1000ms - Orange (slow)"),
            Line::from("  >1000ms    - Red (very slow)"),
            Line::from(""),
            Line::from(vec![
                Span::styled("Press Esc or ? to close", Style::default().fg(Color::DarkGray)),
            ]),
        ];

        let help = Paragraph::new(help_text)
            .block(Block::default().title("Help").borders(Borders::ALL));
        let area = centered_rect(60, 70, size);
        f.render_widget(Clear, area);
        f.render_widget(help, area);
    }

    // Loading indicator
    if app.loading {
        let loading = Paragraph::new("Loading...")
            .block(Block::default().title("Loading").borders(Borders::ALL));
        let area = centered_rect(30, 15, size);
        f.render_widget(Clear, area);
        f.render_widget(loading, area);
    }

    // sing-box not found warning
    if app.singbox_status == SingBoxStatus::NotFound && app.show_help {
        let warning = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("sing-box not found!", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from("Install from:"),
            Line::from(vec![
                Span::styled(SINGBOX_GITHUB, Style::default().fg(Color::Cyan)),
            ]),
        ])
        .block(Block::default().title("Warning").borders(Borders::ALL));
        let area = centered_rect(60, 20, size);
        f.render_widget(Clear, area);
        f.render_widget(warning, area);
    }
}

fn format_proxy_type(ty: &crate::models::ProxyType) -> &'static str {
    match ty {
        crate::models::ProxyType::Shadowsocks => "SS",
        crate::models::ProxyType::Vmess => "VM",
        crate::models::ProxyType::Vless => "VL",
        crate::models::ProxyType::Trojan => "TR",
        crate::models::ProxyType::Hysteria2 => "H2",
        crate::models::ProxyType::Tuic => "TC",
        crate::models::ProxyType::Socks => "SK",
        crate::models::ProxyType::Http => "HT",
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
