mod application;
mod services;

use crate::services::utils;
use application::App;
use libadwaita::gio::prelude::{ApplicationExt, ApplicationExtManual};
use services::config;
use tracing::Level;
use tracing_subscriber::{FmtSubscriber, util::SubscriberInitExt};

fn main() {
    /* Logging */
    let mut log_level = if cfg!(debug_assertions) {
        Level::DEBUG
    } else {
        Level::INFO
    };
    log_level = utils::env::get_log_level().unwrap_or(log_level);
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
