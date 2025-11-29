use anyhow::{Context, Result};
use freedesktop_desktop_entry::DesktopEntry;
use include_dir::{Dir, include_dir};
use std::{
    fs::{self, File},
    io::Write,
    process::Command,
};
use tracing::{debug, info};

use crate::{
    app_dirs::AppDirs,
    config::{self, OnceLockExt},
};

// Calling extract on a subdir does not work and seems bugged.
// Using indivudal imports.
// Also need to fully recompole when the dir changes
static CONFIG: Dir = include_dir!("$CARGO_MANIFEST_DIR/../../assets/config");
static ICON: &[u8] = include_bytes!("../../../assets/app-icon.png");
static DESKTOP_FILE: &str = include_str!("../../../assets/app.desktop");

pub fn init(app_dirs: &AppDirs) -> Result<()> {
    info!("Creating / overwriting assets");
    extract_config_dir(app_dirs)?;
    install_app_icon(app_dirs)?;
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
    let app_version = config::VERSION.get_value();
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
    base_desktop_file.add_desktop_entry("Version".to_string(), app_version.clone());
    base_desktop_file.add_desktop_entry("Icon".to_string(), app_id.clone());
    base_desktop_file.add_desktop_entry("StartupWMClass".to_string(), app_id.clone());

    Ok(base_desktop_file)
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

// TODO add versioning
fn install_app_icon(app_dirs: &AppDirs) -> Result<()> {
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

    // app.add_icon_search_path(&icons_dir);

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
    let app_version = config::VERSION.get_value();
    let user_data_dir = app_dirs.user_data();
    let extension = "desktop";
    let file_name = format!("{app_id}.{extension}");
    let applications_dir = user_data_dir.join("applications");
    let desktop_file_path = applications_dir.join(file_name);

    if desktop_file_path.is_file() && !cfg!(debug_assertions) {
        let existing_desktop_file_str = fs::read_to_string(&desktop_file_path).context(format!(
            "Failed to read existing desktop file: {}",
            &desktop_file_path.display()
        ))?;
        let existing_desktop_file = DesktopEntry::from_str(
            &desktop_file_path,
            &existing_desktop_file_str,
            None::<&[String]>,
        )
        .context(format!(
            "Failed to parse base desktop file: {existing_desktop_file_str:?}"
        ))?;

        if existing_desktop_file
            .version()
            .is_some_and(|version| version == app_version)
        {
            return Ok(());
        }
    }

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
