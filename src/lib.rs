// singboxer library

use anyhow::Result as anyhowResult;

pub mod config;
pub mod latency;
pub mod models;
pub mod parser;
pub mod singbox;

pub use config::{AppConfig, generate_singbox_config, save_singbox_config};
pub use latency::{test_proxy_latency, test_proxies_concurrent, format_latency, latency_color};
pub use models::*;
pub use parser::*;
pub use singbox::{SingBoxManager, SingBoxStatus, SINGBOX_GITHUB};

// App struct for subscription management
pub struct App {
    pub config: AppConfig,
    pub subscriptions: Vec<Subscription>,
    pub proxies: Vec<ProxyServer>,
    pub singbox: SingBoxManager,
}

impl App {
    pub fn new() -> anyhowResult<Self> {
        let config = AppConfig::default();
        config.init()?;

        let subscriptions = config.load_subscriptions().unwrap_or_default();

        Ok(Self {
            config,
            subscriptions,
            proxies: Vec::new(),
            singbox: SingBoxManager::new(),
        })
    }

    pub fn add_subscription(&mut self, name: String, url: String) {
        let new_sub = Subscription::new(name, url, SubscriptionType::Auto);
        self.subscriptions.push(new_sub);
        self.config.save_subscriptions(&self.subscriptions).ok();
    }

    pub fn parse_subscription_content(content: &str, subscription: &Subscription) -> anyhowResult<Vec<ProxyServer>> {
        parser::parse_subscription_content(content, subscription)
    }
}
