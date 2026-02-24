use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::Level;

#[derive(Parser)]
#[command(name = "singboxer")]
#[command(about = "A TUI for managing sing-box configurations", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Launch the TUI
    Ui {},
    /// Add a subscription
    Add {
        /// Subscription name
        name: String,
        /// Subscription URL
        url: String,
    },
    /// Remove a subscription by name
    Remove {
        /// Subscription name to remove
        name: String,
    },
    /// List subscriptions
    List {},
    /// Fetch and display proxies from a subscription
    Fetch {
        /// Subscription URL
        url: String,
    },
    /// Generate sing-box config from subscription
    Generate {
        /// Subscription URL
        url: String,
        /// Output file path
        #[arg(short, long)]
        output: Option<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Setup logging
    let level = if cli.debug { Level::DEBUG } else { Level::INFO };
    tracing_subscriber::fmt()
        .with_max_level(level)
        .init();

    match cli.command {
        Some(Commands::Ui {}) => {
            let app = singboxer::App::new()?;
            singboxer::run_ui(app)?;
        }
        Some(Commands::Add { name, url }) => {
            let mut app = singboxer::App::new()?;
            app.add_subscription(name, url);
            println!("Subscription added successfully.");
        }
        Some(Commands::Remove { name }) => {
            let mut app = singboxer::App::new()?;
            let original_len = app.subscriptions.len();
            app.subscriptions.retain(|s| s.name != name);
            if app.subscriptions.len() < original_len {
                app.config.save_subscriptions(&app.subscriptions)?;
                println!("Removed subscription: {}", name);
            } else {
                println!("Subscription not found: {}", name);
            }
        }
        Some(Commands::List {}) => {
            let app = singboxer::App::new()?;
            if app.subscriptions.is_empty() {
                println!("No subscriptions.");
            } else {
                println!("Subscriptions:");
                for (i, sub) in app.subscriptions.iter().enumerate() {
                    println!("  {}. {} | {:?}", i + 1, sub.name, sub.sub_type);
                }
            }
        }
        Some(Commands::Fetch { url }) => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async {
                match singboxer::fetch_subscription(&url).await {
                    Ok(content) => {
                        let proxies = singboxer::App::parse_subscription_content(&content, &singboxer::models::Subscription {
                            name: "temp".to_string(),
                            url: url.clone(),
                            sub_type: singboxer::models::SubscriptionType::Auto,
                            enabled: true,
                        })?;
                        println!("Found {} proxies:", proxies.len());
                        for proxy in &proxies {
                            println!("  - {} | {} | {}", proxy.name, format_proxy_type(&proxy.proxy_type), proxy.server);
                        }
                        Ok(())
                    }
                    Err(e) => {
                        eprintln!("Error fetching subscription: {}", e);
                        Err(e)
                    }
                }
            })?;
        }
        Some(Commands::Generate { url, output }) => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async {
                match singboxer::fetch_subscription(&url).await {
                    Ok(content) => {
                        let proxies = singboxer::App::parse_subscription_content(&content, &singboxer::models::Subscription {
                            name: "temp".to_string(),
                            url: url.clone(),
                            sub_type: singboxer::models::SubscriptionType::Auto,
                            enabled: true,
                        })?;

                        let config = singboxer::generate_singbox_config(&proxies, None)?;

                        let output_path = output.unwrap_or_else(|| "config.json".to_string());
                        std::fs::write(&output_path, serde_json::to_string_pretty(&config)?)?;
                        println!("Config saved to: {}", output_path);
                        println!("  - {} proxies configured", proxies.len());
                        Ok(())
                    }
                    Err(e) => {
                        eprintln!("Error generating config: {}", e);
                        Err(e)
                    }
                }
            })?;
        }
        None => {
            // Default to UI
            let app = singboxer::App::new()?;
            singboxer::run_ui(app)?;
        }
    }

    Ok(())
}

fn format_proxy_type(ty: &singboxer::models::ProxyType) -> &'static str {
    match ty {
        singboxer::models::ProxyType::Shadowsocks => "SS",
        singboxer::models::ProxyType::Vmess => "VMess",
        singboxer::models::ProxyType::Vless => "VLESS",
        singboxer::models::ProxyType::Trojan => "Trojan",
        singboxer::models::ProxyType::Hysteria2 => "Hysteria2",
        singboxer::models::ProxyType::Tuic => "TUIC",
        singboxer::models::ProxyType::Socks => "SOCKS",
        singboxer::models::ProxyType::Http => "HTTP",
    }
}
