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
    /// Start sing-box with selected proxy
    Start {
        /// Start in foreground (shows logs)
        #[arg(long)]
        foreground: bool,
    },
    /// Stop sing-box
    Stop {},
    /// Check sing-box status
    Status {},
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
        Some(Commands::Import { name, file }) => {
            let content = std::fs::read_to_string(&file)
                .map_err(|e| anyhow::anyhow!("Failed to read file: {}", e))?;

            let proxies = singboxer::App::parse_subscription_content(&content, &singboxer::models::Subscription {
                name: name.clone(),
                url: format!("file://{}", file),
                sub_type: singboxer::models::SubscriptionType::Auto,
                enabled: true,
            })?;

            println!("Imported {} proxies from file:", proxies.len());
            for proxy in &proxies {
                println!("  - {} | {} | {}", proxy.name, format_proxy_type(&proxy.proxy_type), proxy.server);
            }

            // Save as a subscription (stored as file:// URL for reloading)
            let mut app = singboxer::App::new()?;
            app.add_subscription(name, format!("file://{}", file));
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
        Some(Commands::Start { foreground }) => {
            let rt = tokio::runtime::Runtime::new()?;
            let mut app = singboxer::App::new()?;

            // Auto-load proxies from first enabled subscription if none loaded
            if app.proxies.is_empty() {
                if let Some(first_sub) = app.subscriptions.iter().find(|s| s.enabled).cloned() {
                    println!("Loading proxies from: {}", first_sub.name);
                    match rt.block_on(singboxer::fetch_subscription(&first_sub.url)) {
                        Ok(content) => {
                            match singboxer::App::parse_subscription_content(&content, &first_sub) {
                                Ok(proxies) => {
                                    app.proxies = proxies;
                                    println!("Loaded {} proxies.", app.proxies.len());
                                }
                                Err(e) => {
                                    eprintln!("Error parsing subscription: {}", e);
                                    return Err(e);
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Error fetching subscription: {}", e);
                            return Err(e);
                        }
                    }
                } else {
                    eprintln!("Error: No subscriptions found. Add one first:");
                    eprintln!("  ./singboxer add \"Name\" \"https://subscription-url\"");
                    eprintln!("  ./singboxer import \"Name\" \"/path/to/file.yaml\"");
                    return Ok(());
                }
            }

            if app.proxies.is_empty() {
                eprintln!("Error: No proxies found in subscriptions.");
                return Ok(());
            }

            // Generate config and start sing-box
            let config = singboxer::generate_singbox_config(&app.proxies, None)?;
            let config_path = app.config.singbox_config_dir.join("config.json");

            // Save config
            std::fs::create_dir_all(&app.config.singbox_config_dir)?;
            std::fs::write(&config_path, serde_json::to_string_pretty(&config)?)?;

            // Start sing-box
            match app.singbox.start(config_path.to_str().unwrap()) {
                Ok(pid) => {
                    println!("sing-box started successfully (PID: {})", pid);
                    println!("Config: {}", config_path.display());
                    println!("\nTo stop it later, run: ./singboxer stop");

                    if foreground {
                        println!("\nRunning in foreground mode not yet implemented.");
                        println!("sing-box is running in background.");
                    }
                }
                Err(e) => {
                    eprintln!("Error starting sing-box: {}", e);
                    eprintln!("\nTroubleshooting:");
                    eprintln!("1. Make sure sing-box is installed: which sing-box");
                    eprintln!("2. For TUN mode, you may need CAP_NET_ADMIN:");
                    eprintln!("   sudo setcap cap_net_admin,cap_net_raw+ep $(which sing-box)");
                    eprintln!("3. Or run with sudo: sudo ./sing-box start");
                }
            }
        }
        Some(Commands::Stop {}) => {
            let app = singboxer::App::new()?;
            match app.singbox.stop() {
                Ok(_) => println!("sing-box stopped."),
                Err(e) => {
                    eprintln!("Error stopping sing-box: {}", e);
                    eprintln!("It may not be running.");
                }
            }
        }
        Some(Commands::Status {}) => {
            let app = singboxer::App::new()?;
            let status = app.singbox.status();

            match status {
                singboxer::singbox::SingBoxStatus::NotFound => {
                    println!("sing-box: Not Installed");
                    println!("Install from: https://github.com/SagerNet/sing-box#installation");
                }
                singboxer::singbox::SingBoxStatus::Available => {
                    println!("sing-box: Installed (Stopped)");
                }
                singboxer::singbox::SingBoxStatus::Running { pid } => {
                    println!("sing-box: Running (PID: {})", pid);
                }
                singboxer::singbox::SingBoxStatus::Stopped => {
                    println!("sing-box: Stopped");
                }
            }
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
