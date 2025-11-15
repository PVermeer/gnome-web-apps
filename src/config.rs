use libadwaita::gtk::License;

pub const APP_ID: &str = "org.pvermeer.gnome-web-apps";
pub const VERSION: &str = "0.0.0";
pub const APP_NAME: &str = "Gnome Web Apps";
pub const APP_NAME_PATH: &str = "gnome-web-apps";
pub const APP_NAME_CRATE: &str = "gnome_web_apps";
pub const APP_NAME_SHORT: &str = "gwa";
pub const DEVELOPER: &str = "PVermeer";
pub const CREDITS: &[&str] = &["Some credits"];
pub const ACKNOWLEDGEMENT: &[&str] = &["Some acknowledgement"];
pub const LICENSE: License = License::Gpl30;

pub struct DesktopFile {}
impl DesktopFile {
    pub const GWA_KEY: &str = "X-GWA";
    pub const URL_KEY: &str = "X-GWA-URL";
    pub const ID_KEY: &str = "X-GWA-ID";
    pub const BROWSER_ID_KEY: &str = "X-GWA-BROWSER-ID";
    pub const ISOLATE_KEY: &str = "X-GWA-ISOLATE";

    pub const NAME_REPLACE: &str = "%{name}";
    pub const COMMAND_REPLACE: &str = "%{command}";
    pub const URL_REPLACE: &str = "%{url}";
    pub const DOMAIN_REPLACE: &str = "%{domain}";
    pub const ICON_REPLACE: &str = "%{icon}";
}
