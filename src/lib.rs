// singboxer library

pub mod config;
pub mod latency;
pub mod models;
pub mod parser;
pub mod singbox;
pub mod tui;

pub use config::{AppConfig, generate_singbox_config, save_singbox_config};
pub use latency::{test_proxy_latency, test_proxies_concurrent, format_latency, latency_color};
pub use models::*;
pub use parser::*;
pub use singbox::{SingBoxManager, SingBoxStatus, SINGBOX_GITHUB};
pub use tui::{App, run_ui};
