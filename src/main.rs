mod application;
mod config;

use application::App;
use env_logger::Env;
use libadwaita::gio::prelude::{ApplicationExt, ApplicationExtManual};

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let adw_application = libadwaita::Application::builder()
        .application_id(config::APP_ID)
        .build();

    adw_application.connect_activate(|adw_application| {
        App::new(adw_application).init();
    });

    adw_application.run();
}
