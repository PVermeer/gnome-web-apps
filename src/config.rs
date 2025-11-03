use libadwaita::gtk::License;

pub const APP_ID: &str = "org.pvermeer.gnome-web-apps";
pub const VERSION: &str = "0.0.0";
pub const APP_NAME: &str = "Gnome Web Apps";
pub const APP_NAME_PATH: &str = "gnome-web-apps";
pub const APP_NAME_CRATE: &str = "gnome_web_apps";
pub const DEVELOPER: &str = "PVermeer";
pub const CREDITS: &[&str] = &["Some credits"];
pub const ACKNOWLEDGEMENT: &[&str] = &["Some acknowledgement"];
pub const LICENSE: License = License::Gpl30;

pub struct DesktopFile {}
impl DesktopFile {
    pub const GWA_KEY: &str = "X-GWA";
    pub const URL_KEY: &str = "X-GWA-URL";
}
