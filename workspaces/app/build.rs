use anyhow::{Context, Result};
use common::{
    app_dirs::{self, AppDirs},
    assets,
    config::{self, OnceLockExt},
};
use std::{
    fs::{self, File},
    io::Write,
};

fn main() -> Result<()> {
    println!("cargo:warning=Debug: build script is running!");
    config::init();
    let app_dirs = app_dirs::AppDirs::new();
    app_dirs.init()?;

    create_app_desktop_file(&app_dirs)?;
    create_app_icon()?;

    Ok(())
}

fn create_app_desktop_file(app_dirs: &AppDirs) -> Result<()> {
    let desktop_file = assets::create_app_desktop_file(app_dirs)?;
    let file_name = desktop_file
        .path
        .file_name()
        .context("Failed to get filename on dekstop file")?;
    let save_dir = AppDirs::build_assets_desktop_path().unwrap();
    let save_path = save_dir.join(file_name);

    if !save_dir.is_dir() {
        fs::create_dir_all(&save_dir)?;
    }

    fs::write(&save_path, desktop_file.to_string())?;

    Ok(())
}

fn create_app_icon() -> Result<()> {
    let app_id = config::APP_ID.get_value();
    let extension = "png";
    let file_name = format!("{app_id}.{extension}");
    let save_dir = AppDirs::build_assets_desktop_path().unwrap();
    let save_path = save_dir.join(file_name);

    if !save_dir.is_dir() {
        fs::create_dir_all(&save_dir)?;
    }

    let mut icon_file = File::create(&save_path)?;
    icon_file.write_all(assets::get_icon_data())?;

    Ok(())
}
