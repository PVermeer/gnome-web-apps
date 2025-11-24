mod application;
mod services;

use anyhow::Context;
use application::App;
use libadwaita::gio::prelude::{ApplicationExt, ApplicationExtManual};
use services::config;
use std::str::FromStr;
use tracing::Level;
use tracing_subscriber::{FmtSubscriber, util::SubscriberInitExt};

fn main() {
    /* Logging */
    // Max level for debug builds
    let max_level = if cfg!(debug_assertions) {
        Level::TRACE
    } else {
        Level::INFO
    };
    let log_level = std::env::var("RUST_LOG")
        .with_context(|| {
            let info = format!("No LOG environment variable set, using '{max_level}'");
            println!("{info}");
            info
        })
        .and_then(|level_str| {
            Level::from_str(&level_str).with_context(|| {
                let error = format!("Invalid LOG environment variable set, using '{max_level}'");
                eprintln!("{error}");
                error
            })
        })
        .unwrap_or(max_level);

    // Disable > info logging for external crates
    let filter = format!("{}={}", config::APP_NAME_CRATE, log_level);
    let logger = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_env_filter(filter)
        .finish();
    logger.init();

    let adw_application = libadwaita::Application::default();

    adw_application.connect_activate(|adw_application| {
        App::new(adw_application).init();
    });

    adw_application.run();
}
