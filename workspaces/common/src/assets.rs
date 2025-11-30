use crate::{
    app_dirs::AppDirs,
    config::{self, OnceLockExt},
};
use anyhow::{Context, Result};
use freedesktop_desktop_entry::DesktopEntry;
use gtk::IconTheme;
use include_dir::{Dir, include_dir};
use std::{
    fs::{self, File},
    io::Write,
    process::Command,
};
use tracing::{debug, info};

// Calling extract on a subdir does not work and seems bugged.
// Using indivudal imports.
// Also need to fully recompole when the dir changes
static CONFIG: Dir = include_dir!("$CARGO_MANIFEST_DIR/../../assets/config");
static ICON: &[u8] = include_bytes!("../../../assets/app-icon.png");
static DESKTOP_FILE: &str = include_str!("../../../assets/app.desktop");
static META_INFO: &str = include_str!("../../../assets/app.metainfo.xml");

pub fn init(app_dirs: &AppDirs, icon_theme: &IconTheme) -> Result<()> {
    info!("Creating / overwriting assets");
    extract_config_dir(app_dirs)?;
    install_app_icon(app_dirs, icon_theme)?;
    install_desktop_file(app_dirs)?;
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

pub fn create_app_desktop_file(app_dirs: &AppDirs) -> Result<DesktopEntry> {
    let app_id = config::APP_ID.get_value();
    let app_name = config::APP_NAME.get_value().clone();
    let user_data_dir = app_dirs.user_data();
    let extension = "desktop";
    let file_name = format!("{app_id}.{extension}");
    let applications_dir = user_data_dir.join("applications");
    let desktop_file_path = applications_dir.join(file_name);

    let mut base_desktop_file =
        DesktopEntry::from_str(&desktop_file_path, DESKTOP_FILE, None::<&[String]>).context(
            format!("Failed to parse base desktop file: {DESKTOP_FILE:?}"),
        )?;

    base_desktop_file.add_desktop_entry("Name".to_string(), app_name.clone());
    base_desktop_file.add_desktop_entry("Icon".to_string(), app_id.clone());
    base_desktop_file.add_desktop_entry("StartupWMClass".to_string(), app_id.clone());

    Ok(base_desktop_file)
}

pub fn get_icon_data() -> &'static [u8] {
    ICON
}

pub fn get_meta_info() -> &'static str {
    META_INFO
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

fn install_app_icon(app_dirs: &AppDirs, icon_theme: &IconTheme) -> Result<()> {
    debug!("Installing app icon");

    let app_id = config::APP_ID.get_value();
    let user_data_dir = app_dirs.user_data();
    let extension = "png";
    let file_name = format!("{app_id}.{extension}");
    let icons_dir = user_data_dir.join("icons");
    let icon_save_dir = icons_dir.join("hicolor").join("256x256").join("apps");
    let icon_path = icon_save_dir.join(file_name);

    if !icon_save_dir.is_dir() {
        fs::create_dir_all(&icon_save_dir).context(format!(
            "Failed to create icon dir: {}",
            icon_save_dir.display()
        ))?;
    }

    let mut icon_file = File::create(&icon_path).context(format!(
        "Failed to create icon file: {}",
        icon_path.display()
    ))?;

    debug!("Saving app icon to: {}", icon_path.display());
    icon_file
        .write_all(ICON)
        .context("Failed to write new icon file")?;

    icon_theme.add_search_path(&icons_dir);

    debug!("Running command icon update command host");
    let result = Command::new("xdg-icon-resource").arg("forceupdate").spawn();
    if let Err(error) = result {
        debug!(
            error = error.to_string(),
            "Failed to run icon update command on host"
        );
    }

    Ok(())
}

fn install_desktop_file(app_dirs: &AppDirs) -> Result<()> {
    let app_id = config::APP_ID.get_value();
    let user_data_dir = app_dirs.user_data();
    let extension = "desktop";
    let file_name = format!("{app_id}.{extension}");
    let applications_dir = user_data_dir.join("applications");
    let desktop_file_path = applications_dir.join(file_name);

    debug!("Installing desktop file");

    let desktop_file = create_app_desktop_file(app_dirs)?;

    debug!(
        "Saving app desktop file to: {}",
        &desktop_file_path.display()
    );
    fs::write(&desktop_file_path, desktop_file.to_string()).context(format!(
        "Failed to write new app desktop file to fs: {}",
        desktop_file.path.display()
    ))?;

    Ok(())
}
