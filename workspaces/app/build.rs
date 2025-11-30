use anyhow::{Context, Result};
use common::{
    app_dirs::AppDirs,
    assets,
    config::{self, OnceLockExt},
    utils,
};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Serialize, Deserialize, PartialEq)]
struct MetaInfoXml {}

fn main() -> Result<()> {
    println!("cargo:warning=Debug: build script is running!");
    config::init();
    let app_dirs = AppDirs::new();
    app_dirs.init()?;

    create_config_symlinks(&app_dirs);
    create_data_symlinks(&app_dirs);

    create_app_desktop_file(&app_dirs)?;
    create_app_icon()?;
    create_app_metainfo_file()?;

    Ok(())
}

fn create_config_symlinks(app_dirs: &AppDirs) {
    let config_path = build_dev_config_path();
    let _ = utils::files::create_symlink(&config_path, &app_dirs.config());
}

fn create_data_symlinks(app_dirs: &AppDirs) {
    let data_path = build_dev_data_path();
    let assets_desktop_path = build_dev_assets_path().join("desktop-files");

    let _ = utils::files::create_symlink(&data_path, &app_dirs.data());
    let _ = utils::files::create_symlink(&data_path.join("applications"), &app_dirs.applications());

    for file in utils::files::get_entries_in_dir(&assets_desktop_path).unwrap() {
        if file.path().extension().unwrap_or_default() != "desktop" {
            continue;
        }
        let _ = utils::files::create_symlink(
            &app_dirs.applications().join(file.file_name()),
            &file.path(),
        );
    }
}

fn create_app_desktop_file(app_dirs: &AppDirs) -> Result<()> {
    let desktop_file = assets::create_app_desktop_file(app_dirs)?;
    let file_name = desktop_file
        .path
        .file_name()
        .context("Failed to get filename on dekstop file")?;
    let save_dir = build_assets_path().join("desktop");
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
    let save_dir = build_assets_path().join("desktop");
    let save_path = save_dir.join(file_name);

    if !save_dir.is_dir() {
        fs::create_dir_all(&save_dir)?;
    }

    let mut icon_file = File::create(&save_path)?;
    icon_file.write_all(assets::get_icon_data())?;

    Ok(())
}

fn create_app_metainfo_file() -> Result<()> {
    let meta_info: MetaInfoXml = serde_xml::from_str(assets::get_meta_info())?;
    let save_dir = build_assets_path().join("desktop");
    let save_path = save_dir.join(format!("{}.metadata.xml", config::APP_ID.get_value()));

    let serialised = serde_xml::to_string(&meta_info)?;

    if !save_dir.is_dir() {
        fs::create_dir_all(&save_dir)?;
    }
    fs::write(&save_path, serialised)?;

    Ok(())
}

fn project_path() -> PathBuf {
    Path::new("").join("..").join("..").canonicalize().unwrap()
}

fn build_assets_path() -> PathBuf {
    project_path().join("assets")
}

fn build_dev_assets_path() -> PathBuf {
    project_path().join("dev-assets")
}

fn build_dev_config_path() -> PathBuf {
    project_path().join("dev-config")
}

fn build_dev_data_path() -> PathBuf {
    project_path().join("dev-data")
}
