use crate::{
    application::App,
    services::config::{self, OnceLockExt},
};
use anyhow::{Context, Result};
use include_dir::{Dir, include_dir};
use std::{
    fs::{self, File},
    io::Write,
    rc::Rc,
};
use tracing::{debug, info};

// Calling extract on a subdir does not work and seems bugged.
// Using indivudal imports.
static CONFIG: Dir = include_dir!("$CARGO_MANIFEST_DIR/assets/config");
static ICON: &[u8; 200_202] = include_bytes!("../../assets/app-icon.png");

pub fn init(app: &Rc<App>) -> Result<()> {
    info!("Creating / overwriting assets");
    extract_config_dir(app)?;
    extract_app_icon(app)?;
    Ok(())
}

pub fn reset_config_files(app: &Rc<App>) -> Result<()> {
    let config_dir = app.dirs.config();

    if config_dir.is_dir() {
        info!("Deleting config files");
        fs::remove_dir_all(config_dir)?;
    }

    extract_config_dir(app)?;

    Ok(())
}

fn extract_config_dir(app: &Rc<App>) -> Result<()> {
    debug!("Extracting config dir");
    let config_dir = app.dirs.config();

    CONFIG.extract(&config_dir).context(format!(
        "Failed to extract config dir from ASSETS in: {}",
        &config_dir.display()
    ))?;

    Ok(())
}

fn extract_app_icon(app: &Rc<App>) -> Result<()> {
    let app_id = config::APP_ID.get_value();
    if app.has_icon(app_id) {
        return Ok(());
    }
    debug!("Extracting app icon");

    let user_data_dir = app.dirs.user_data();
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

    let mut icon_file = File::create_new(&icon_path).context(format!(
        "Failed to create icon file: {}",
        icon_path.display()
    ))?;

    icon_file
        .write_all(ICON)
        .context("Failed to write new icon file")?;

    app.add_icon_search_path(&icons_dir);

    Ok(())
}
