use anyhow::{Context, Result};
use common::{
    app_dirs::AppDirs,
    config::{self, OnceLockExt},
    utils,
};
use std::{
    fs::{self},
    path::{Path, PathBuf},
};

fn main() -> Result<()> {
    println!("cargo:warning=Debug: build script is running!");
    config::init();
    let app_dirs = AppDirs::new();
    app_dirs.init()?;

    create_config_symlinks(&app_dirs);
    create_data_symlinks(&app_dirs);
    copy_dev_web_apps(&app_dirs);

    install_app_desktop_file(&app_dirs)?;
    install_app_icon(&app_dirs)?;

    Ok(())
}

fn create_config_symlinks(app_dirs: &AppDirs) {
    let config_path = dev_config_path();
    let _ = utils::files::create_symlink(&config_path, &app_dirs.config());
}

fn create_data_symlinks(app_dirs: &AppDirs) {
    let data_path = dev_data_path();

    let _ = utils::files::create_symlink(&data_path, &app_dirs.data());
    let _ = utils::files::create_symlink(&data_path.join("applications"), &app_dirs.applications());
}

fn copy_dev_web_apps(app_dirs: &AppDirs) {
    let dev_desktop_files = dev_assets_path().join("desktop-files");
    let user_applications_dir = app_dirs.applications();

    for desktop_file in &utils::files::get_entries_in_dir(&dev_desktop_files).unwrap() {
        let id = desktop_file
            .file_name()
            .to_string_lossy()
            .split('-')
            .next_back()
            .unwrap()
            .to_string();

        let mut exists = false;
        for file in &utils::files::get_entries_in_dir(&user_applications_dir).unwrap() {
            if file.file_name().to_string_lossy().ends_with(&id) {
                exists = true;
            }
        }
        if exists {
            continue;
        }

        fs::copy(
            desktop_file.path(),
            user_applications_dir.join(desktop_file.file_name()),
        )
        .unwrap();
    }
}

fn install_app_desktop_file(app_dirs: &AppDirs) -> Result<()> {
    let file_name = desktop_file_name();
    let desktop_file = assets_path().join("desktop").join(&file_name);
    let save_file = app_dirs.applications().join(file_name);

    fs::copy(desktop_file, save_file).context("Desktop file copy failed")?;
    Ok(())
}

fn install_app_icon(app_dirs: &AppDirs) -> Result<()> {
    let file_name = icon_file_name();
    let icon_file = assets_path().join("desktop").join(&file_name);
    let save_dir = app_dirs
        .user_data()
        .join("icons")
        .join("hicolor")
        .join("256x256")
        .join("apps");
    if !save_dir.is_dir() {
        fs::create_dir_all(&save_dir).context("Failed to create icon dir")?;
    }

    let save_file = save_dir.join(file_name);

    fs::copy(icon_file, save_file).context("Icon copy failed")?;
    Ok(())
}

fn project_path() -> PathBuf {
    Path::new("").join("..").join("..").canonicalize().unwrap()
}

fn assets_path() -> PathBuf {
    project_path().join("assets")
}

fn dev_config_path() -> PathBuf {
    project_path().join("dev-config")
}

fn dev_data_path() -> PathBuf {
    project_path().join("dev-data")
}

fn dev_assets_path() -> PathBuf {
    project_path().join("dev-assets")
}

fn desktop_file_name() -> String {
    let app_id = config::APP_ID.get_value();
    let extension = "desktop";
    let file_name = format!("{app_id}.{extension}");

    file_name
}

fn icon_file_name() -> String {
    let app_id = config::APP_ID.get_value();
    let extension = "png";
    let file_name = format!("{app_id}.{extension}");

    file_name
}
