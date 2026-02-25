use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::Level;

#[derive(Parser)]
#[command(name = "singboxer")]
#[command(about = "CLI tool for managing sing-box configurations", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a subscription from URL
    Add {
        /// Subscription name
        name: String,
        /// Subscription URL
        url: String,
    },
    /// Import a subscription from file
    Import {
        /// Subscription name
        name: String,
        /// Path to subscription file (Clash YAML, etc.)
        file: String,
    },
    /// Remove a subscription by name
    Remove {
        /// Subscription name to remove
        name: String,
    },
    /// List subscriptions
    List {},
    /// List all proxies from subscriptions
    Proxies {
        /// Filter by subscription name
        subscription: Option<String>,
    },
    /// Test latency for all proxies
    Test {
        /// Subscription name to test (default: all)
        subscription: Option<String>,
    },
    /// Select a proxy to use
    Select {
        /// Proxy name (partial match allowed)
        name: String,
    },
    /// Get current selected proxy
    Current {},
    /// Start sing-box
    Start {
        /// Disable TUN mode (SOCKS/HTTP only)
        #[arg(long)]
        no_tun: bool,
    },
    /// Stop sing-box
    Stop {},
    /// Check sing-box status
    Status {},
    /// Generate sing-box config from subscription
    Generate {
        /// Subscription name or URL
        source: String,
        /// Output file path
        #[arg(short, long)]
        output: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Setup logging
    let level = if cli.debug { Level::DEBUG } else { Level::INFO };
    tracing_subscriber::fmt()
        .with_max_level(level)
        .init();

    match cli.command {
        Some(Commands::Add { name, url }) => {
            command_add(name, url).await?;
        }
        Some(Commands::Import { name, file }) => {
            command_import(name, file).await?;
        }
        Some(Commands::Remove { name }) => {
            command_remove(name)?;
        }
        Some(Commands::List {}) => {
            command_list()?;
        }
        Some(Commands::Proxies { subscription }) => {
            command_proxies(subscription).await?;
        }
        Some(Commands::Test { subscription }) => {
            command_test(subscription).await?;
        }
        Some(Commands::Select { name }) => {
            command_select(name).await?;
        }
        Some(Commands::Current {}) => {
            command_current().await?;
        }
        Some(Commands::Start { no_tun }) => {
            command_start(no_tun).await?;
        }
        Some(Commands::Stop {}) => {
            command_stop()?;
        }
        Some(Commands::Status {}) => {
            command_status()?;
        }
        Some(Commands::Generate { source, output }) => {
            command_generate(source, output).await?;
        }
        None => {
            print_help();
        }
    }

    Ok(())
}

fn print_help() {
    println!("singboxer - CLI tool for managing sing-box configurations");
    println!();
    println!("Quick start:");
    println!("  1. Add a subscription:  singboxer add \"my-sub\" \"https://url\"");
    println!("  2. List proxies:        singboxer proxies");
    println!("  3. Test latency:        singboxer test");
    println!("  4. Select proxy:        singboxer select \"proxy-name\"");
    println!("  5. Start sing-box:      singboxer start");
    println!();
    println!("Run 'singboxer help' for all commands.");
}

async fn command_add(name: String, url: String) -> Result<()> {
    let config = get_config()?;
    config.init()?;

    let subs = config.load_subscriptions()?;

    // Check for duplicate
    if subs.iter().any(|s| s.name == name) {
        eprintln!("Error: Subscription '{}' already exists", name);
        return Ok(());
    }

    let new_sub = singboxer::Subscription::new(
        name.clone(),
        url,
        singboxer::SubscriptionType::Auto,
    );

    let mut new_subs = subs;
    new_subs.push(new_sub);
    config.save_subscriptions(&new_subs)?;

    println!("Added subscription: {}", name);
    Ok(())
}

async fn command_import(name: String, file: String) -> Result<()> {
    let content = std::fs::read_to_string(&file)
        .map_err(|e| anyhow::anyhow!("Failed to read file: {}", e))?;

    let temp_sub = singboxer::Subscription::new(
        name.clone(),
        format!("file://{}", file),
        singboxer::SubscriptionType::Auto,
    );

    let proxies = singboxer::App::parse_subscription_content(&content, &temp_sub)?;

    println!("Imported {} proxies from file:", proxies.len());
    for proxy in &proxies {
        println!("  - {} | {} | {}", proxy.name, format_proxy_type(&proxy.proxy_type), proxy.server);
    }

    // Save as subscription
    command_add(name, format!("file://{}", file)).await?;
    Ok(())
}

fn command_remove(name: String) -> Result<()> {
    let config = get_config()?;
    let mut subs = config.load_subscriptions()?;

    let original_len = subs.len();
    subs.retain(|s| s.name != name);

    if subs.len() < original_len {
        config.save_subscriptions(&subs)?;
        println!("Removed subscription: {}", name);
    } else {
        println!("Subscription not found: {}", name);
    }
    Ok(())
}

fn command_list() -> Result<()> {
    let config = get_config()?;
    let subs = config.load_subscriptions()?;

    if subs.is_empty() {
        println!("No subscriptions.");
        println!("Add one with: singboxer add \"name\" \"url\"");
    } else {
        println!("Subscriptions:");
        for (i, sub) in subs.iter().enumerate() {
            let enabled = if sub.enabled { "" } else { " (disabled)" };
            println!("  {}. {} | {}{}", i + 1, sub.name, format_sub_type(&sub.sub_type), enabled);
        }
    }
    Ok(())
}

async fn command_proxies(subscription: Option<String>) -> Result<()> {
    let config = get_config()?;
    let subs = config.load_subscriptions()?;

    if subs.is_empty() {
        eprintln!("No subscriptions. Add one first.");
        return Ok(());
    }

    let subs_to_fetch: Vec<_> = if let Some(sub_name) = subscription {
        subs.iter().filter(|s| s.name == sub_name).collect()
    } else {
        subs.iter().collect()
    };

    if subs_to_fetch.is_empty() {
        eprintln!("No matching subscription found.");
        return Ok(());
    }

    for sub in subs_to_fetch {
        if !sub.enabled {
            continue;
        }

        match singboxer::fetch_subscription(&sub.url).await {
            Ok(content) => {
                match singboxer::App::parse_subscription_content(&content, sub) {
                    Ok(proxies) => {
                        println!("\n[{}] {} proxies:", sub.name, proxies.len());
                        for (i, proxy) in proxies.iter().enumerate() {
                            let latency = if let Some(ms) = proxy.latency_ms {
                                format!(" | {}", singboxer::format_latency(Some(ms), false))
                            } else {
                                String::new()
                            };
                            println!("  {}. {}{} | {} | {}",
                                i + 1,
                                proxy.name,
                                latency,
                                format_proxy_type(&proxy.proxy_type),
                                proxy.server
                            );
                        }
                    }
                    Err(e) => {
                        eprintln!("Error parsing {}: {}", sub.name, e);
                    }
                }
            }
            Err(e) => {
                eprintln!("Error fetching {}: {}", sub.name, e);
            }
        }
    }

    Ok(())
}

async fn command_test(subscription: Option<String>) -> Result<()> {
    let config = get_config()?;
    let subs = config.load_subscriptions()?;

    if subs.is_empty() {
        eprintln!("No subscriptions. Add one first.");
        return Ok(());
    }

    let subs_to_fetch: Vec<_> = if let Some(sub_name) = subscription {
        subs.iter().filter(|s| s.name == sub_name).collect()
    } else {
        subs.iter().filter(|s| s.enabled).collect()
    };

    if subs_to_fetch.is_empty() {
        eprintln!("No matching subscription found.");
        return Ok(());
    }

    let mut all_proxies: Vec<singboxer::ProxyServer> = Vec::new();

    for sub in subs_to_fetch {
        match singboxer::fetch_subscription(&sub.url).await {
            Ok(content) => {
                match singboxer::App::parse_subscription_content(&content, sub) {
                    Ok(mut proxies) => {
                        println!("Testing {} proxies from {}...", proxies.len(), sub.name);
                        all_proxies.append(&mut proxies);
                    }
                    Err(e) => {
                        eprintln!("Error parsing {}: {}", sub.name, e);
                    }
                }
            }
            Err(e) => {
                eprintln!("Error fetching {}: {}", sub.name, e);
            }
        }
    }

    if all_proxies.is_empty() {
        eprintln!("No proxies to test.");
        return Ok(());
    }

    println!("\nTesting {} proxies...\n", all_proxies.len());

    // Test each proxy sequentially
    let mut results = Vec::new();
    for proxy in &all_proxies {
        let result = singboxer::test_proxy_latency(proxy, None).await;
        let latency = result.ok().map(|d| d.as_millis());
        results.push((proxy.name.clone(), latency));
    }

    // Sort by latency
    results.sort_by_key(|r| r.1);

    println!("Results:");
    for (name, latency) in &results {
        if let Some(ms) = latency {
            let ms_u64: u64 = (*ms).try_into().unwrap_or(u64::MAX);
            println!("  {} | {}", singboxer::format_latency(Some(ms_u64), false), name);
        } else {
            println!("  timeout | {}", name);
        }
    }

    Ok(())
}

async fn command_select(name: String) -> Result<()> {
    // Check if sing-box is running
    let manager = singboxer::SingBoxManager::new();
    manager.check_installation()?;

    let status = manager.status();
    if !matches!(status, singboxer::SingBoxStatus::Running { .. }) {
        eprintln!("Error: sing-box is not running.");
        eprintln!("Start it first: singboxer start");
        return Ok(());
    }

    // Get all proxies to find match
    let config = get_config()?;
    let subs = config.load_subscriptions()?;
    let mut all_proxies: Vec<singboxer::ProxyServer> = Vec::new();

    for sub in &subs {
        if !sub.enabled {
            continue;
        }
        if let Ok(content) = singboxer::fetch_subscription(&sub.url).await {
            if let Ok(proxies) = singboxer::App::parse_subscription_content(&content, sub) {
                all_proxies.extend(proxies);
            }
        }
    }

    // Find matching proxy
    let name_lower = name.to_lowercase();
    let matches: Vec<_> = all_proxies.iter()
        .filter(|p| p.name.to_lowercase().contains(&name_lower))
        .collect();

    if matches.is_empty() {
        eprintln!("Error: No proxy found matching '{}'", name);
        eprintln!("Use 'singboxer proxies' to list available proxies.");
        return Ok(());
    }

    if matches.len() > 1 {
        eprintln!("Multiple proxies match '{}':", name);
        for m in matches {
            eprintln!("  - {}", m.name);
        }
        eprintln!("Please be more specific.");
        return Ok(());
    }

    let selected = matches[0];
    let tag = sanitize_tag(&selected.name);

    println!("Selecting proxy: {}", selected.name);

    manager.switch_proxy(&tag).await?;
    println!("Selected: {}", selected.name);

    Ok(())
}

async fn command_current() -> Result<()> {
    let manager = singboxer::SingBoxManager::new();
    manager.check_installation()?;

    let status = manager.status();
    if !matches!(status, singboxer::SingBoxStatus::Running { .. }) {
        println!("sing-box is not running");
        return Ok(());
    }

    match manager.get_selected_proxy().await {
        Ok(proxy) => println!("Current: {}", proxy),
        Err(e) => println!("Could not determine current proxy: {}", e),
    }

    Ok(())
}

async fn command_start(no_tun: bool) -> Result<()> {
    let config = get_config()?;
    let subs = config.load_subscriptions()?;

    if subs.is_empty() {
        eprintln!("No subscriptions. Add one first:");
        eprintln!("  singboxer add \"name\" \"url\"");
        return Ok(());
    }

    // Load proxies from first enabled subscription
    let mut all_proxies: Vec<singboxer::ProxyServer> = Vec::new();
    for sub in subs.iter().filter(|s| s.enabled) {
        match singboxer::fetch_subscription(&sub.url).await {
            Ok(content) => {
                if let Ok(proxies) = singboxer::App::parse_subscription_content(&content, sub) {
                    println!("Loaded {} proxies from {}", proxies.len(), sub.name);
                    all_proxies.extend(proxies);
                }
            }
            Err(e) => {
                eprintln!("Error fetching {}: {}", sub.name, e);
            }
        }
    }

    if all_proxies.is_empty() {
        eprintln!("No proxies found in subscriptions.");
        return Ok(());
    }

    // Get last selected proxy from state
    let selected = get_last_selected_proxy().await;

    // Generate config
    let singbox_config = singboxer::generate_singbox_config(&all_proxies, selected.as_deref(), no_tun)?;
    let config_path = config.singbox_config_dir.join("config.json");

    // Save config
    std::fs::create_dir_all(&config.singbox_config_dir)?;
    std::fs::write(&config_path, serde_json::to_string_pretty(&singbox_config)?)?;

    // Start sing-box
    let manager = singboxer::SingBoxManager::new();
    match manager.start(config_path.to_str().unwrap()) {
        Ok(pid) => {
            println!("sing-box started (PID: {})", pid);

            if let Some(sel) = selected {
                println!("Selected: {}", sel);
            }

            if no_tun {
                println!("\nRunning without TUN mode.");
                println!("Configure your browser to use:");
                println!("  SOCKS5: 127.0.0.1:7890");
                println!("  HTTP:   127.0.0.1:7891");
            } else {
                println!("\nTUN mode enabled - system proxy configured automatically.");
            }
        }
        Err(e) => {
            eprintln!("Error starting sing-box: {}", e);
            eprintln!("\nTroubleshooting:");
            eprintln!("  macOS: Run with sudo");
            eprintln!("  Linux: sudo setcap cap_net_admin+ep $(which sing-box)");
            if !no_tun {
                eprintln!("  Or use: singboxer start --no-tun");
            }
        }
    }

    Ok(())
}

fn command_stop() -> Result<()> {
    let manager = singboxer::SingBoxManager::new();
    match manager.stop() {
        Ok(_) => println!("sing-box stopped"),
        Err(e) => eprintln!("Error stopping sing-box: {}", e),
    }
    Ok(())
}

fn command_status() -> Result<()> {
    let manager = singboxer::SingBoxManager::new();

    match manager.status() {
        singboxer::SingBoxStatus::NotFound => {
            println!("sing-box: Not installed");
            println!("Get it from: https://github.com/SagerNet/sing-box");
        }
        singboxer::SingBoxStatus::Available => {
            println!("sing-box: Installed (stopped)");
        }
        singboxer::SingBoxStatus::Running { pid } => {
            println!("sing-box: Running (PID: {})", pid);
        }
        singboxer::SingBoxStatus::Stopped => {
            println!("sing-box: Stopped");
        }
    }
    Ok(())
}

async fn command_generate(source: String, output: Option<String>) -> Result<()> {
    // Check if source is a URL or subscription name
    let (proxies, _name) = if source.starts_with("http://") || source.starts_with("https://") || source.starts_with("file://") {
        let temp_sub = singboxer::Subscription::new(
            "temp".to_string(),
            source.clone(),
            singboxer::SubscriptionType::Auto,
        );
        let content = singboxer::fetch_subscription(&source).await?;
        (singboxer::App::parse_subscription_content(&content, &temp_sub)?, source)
    } else {
        // Look up subscription by name
        let config = get_config()?;
        let subs = config.load_subscriptions()?;
        let sub = subs.iter().find(|s| s.name == source)
            .ok_or_else(|| anyhow::anyhow!("Subscription '{}' not found", source))?;
        let content = singboxer::fetch_subscription(&sub.url).await?;
        (singboxer::App::parse_subscription_content(&content, sub)?, sub.name.clone())
    };

    let config = singboxer::generate_singbox_config(&proxies, None, false)?;

    let output_path = output.unwrap_or_else(|| "config.json".to_string());
    std::fs::write(&output_path, serde_json::to_string_pretty(&config)?)?;

    println!("Generated config: {}", output_path);
    println!("  Proxies: {}", proxies.len());

    Ok(())
}

// Helper functions

fn get_config() -> Result<singboxer::AppConfig> {
    Ok(singboxer::AppConfig::default())
}

fn format_proxy_type(ty: &singboxer::ProxyType) -> &'static str {
    match ty {
        singboxer::ProxyType::Shadowsocks => "SS",
        singboxer::ProxyType::Vmess => "VMess",
        singboxer::ProxyType::Vless => "VLESS",
        singboxer::ProxyType::Trojan => "Trojan",
        singboxer::ProxyType::Hysteria2 => "Hysteria2",
        singboxer::ProxyType::Tuic => "TUIC",
        singboxer::ProxyType::Socks => "SOCKS",
        singboxer::ProxyType::Http => "HTTP",
    }
}

fn format_sub_type(ty: &singboxer::SubscriptionType) -> &'static str {
    match ty {
        singboxer::SubscriptionType::Clash => "Clash",
        singboxer::SubscriptionType::Shadowsocks => "Shadowsocks",
        singboxer::SubscriptionType::V2Ray => "V2Ray",
        singboxer::SubscriptionType::Singbox => "Singbox",
        singboxer::SubscriptionType::Auto => "Auto",
    }
}

fn sanitize_tag(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

async fn get_last_selected_proxy() -> Option<String> {
    // Try to get current selection from Clash API
    let manager = singboxer::SingBoxManager::new();
    if manager.status() != singboxer::SingBoxStatus::Available {
        return None;
    }

    // Check if we can get current selection
    manager.get_selected_proxy().await.ok()
}
