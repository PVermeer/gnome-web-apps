use crate::{
    app_dirs::AppDirs,
    config::{self, OnceLockExt},
};
use anyhow::{Context, Result};
use freedesktop_desktop_entry::DesktopEntry;
use include_dir::{Dir, include_dir};
use std::fs::{self};
use tracing::{debug, info};

// Calling extract on a subdir does not work and seems bugged.
// Using indivudal imports.
// Also need to fully recompile when the dir changes
static CONFIG: Dir = include_dir!("$CARGO_MANIFEST_DIR/../../assets/config");
static DESKTOP: Dir = include_dir!("$CARGO_MANIFEST_DIR/../../assets/desktop");
static ICON_IN: &[u8] = include_bytes!("../../../assets/app-icon.png");
static DESKTOP_FILE_IN: &str = include_str!("../../../assets/app.desktop");
static META_INFO_IN: &str = include_str!("../../../assets/app.metainfo.xml");
static APP_DESCRIPTION: &str = include_str!("../../../assets/app-description.markup");

pub fn init(app_dirs: &AppDirs) -> Result<()> {
    info!("Creating / overwriting assets");
    extract_config_dir(app_dirs)?;
    Ok(())
}

pub fn reset_config_files(app_dirs: &AppDirs) -> Result<()> {
    let config_dir = app_dirs.config();

    if config_dir.is_dir() {
        info!("Deleting config files");
        fs::remove_dir_all(config_dir)?;
    }

    extract_config_dir(app_dirs)?;

    Ok(())
}

pub fn create_stand_alone_desktop_file(app_dirs: &AppDirs) -> Result<DesktopEntry> {
    let app_id = config::APP_ID.get_value();
    let app_name = config::APP_NAME.get_value().clone();
    let user_data_dir = app_dirs.user_data();
    let extension = "desktop";
    let file_name = format!("{app_id}.{extension}");
    let applications_dir = user_data_dir.join("applications");
    let desktop_file_path = applications_dir.join(file_name);

    let mut base_desktop_file =
        DesktopEntry::from_str(&desktop_file_path, DESKTOP_FILE_IN, None::<&[String]>).context(
            format!("Failed to parse base desktop file: {DESKTOP_FILE_IN:?}"),
        )?;

    base_desktop_file.add_desktop_entry("Name".to_string(), app_name.clone());
    base_desktop_file.add_desktop_entry("Icon".to_string(), app_id.clone());
    base_desktop_file.add_desktop_entry("StartupWMClass".to_string(), app_id.clone());

    Ok(base_desktop_file)
}

pub fn get_icon_data_in() -> &'static [u8] {
    ICON_IN
}

pub fn get_meta_info_in() -> &'static str {
    META_INFO_IN
}

pub fn get_meta_info() -> &'static str {
    let app_id = config::APP_ID.get_value();
    DESKTOP
        .get_file(format!("{app_id}.metainfo.xml"))
        .and_then(|file| file.contents_utf8())
        .unwrap_or_default()
}

pub fn get_app_description() -> &'static str {
    APP_DESCRIPTION
}

pub fn get_desktop_file_in() -> &'static str {
    DESKTOP_FILE_IN
}

fn extract_config_dir(app_dirs: &AppDirs) -> Result<()> {
    debug!("Extracting config dir");
    let config_dir = app_dirs.config();

    CONFIG.extract(&config_dir).context(format!(
        "Failed to extract config dir from ASSETS in: {}",
        &config_dir.display()
    ))?;

    Ok(())
}
