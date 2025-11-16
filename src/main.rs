mod application;
mod ext;
mod services;

use application::App;
use env_logger::Env;
use libadwaita::gio::prelude::{ApplicationExt, ApplicationExtManual};
use log::LevelFilter;
use services::config;

fn main() {
    let mut logger = env_logger::Builder::from_env(Env::default().default_filter_or("info"));
    if cfg!(debug_assertions) {
        // Only enable debug logging for this app
        logger
            .filter_level(LevelFilter::Info)
            .filter_module(config::APP_NAME_CRATE, LevelFilter::max());
    }
    logger.init();

    let adw_application = libadwaita::Application::builder()
        .application_id(config::APP_ID)
        .build();

    adw_application.connect_activate(|adw_application| {
        App::new(adw_application).init();
    });

    adw_application.run();
}
