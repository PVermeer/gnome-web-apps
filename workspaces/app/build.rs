use anyhow::{Result, bail};
use common::{
    app_dirs::AppDirs,
    assets,
    config::{self, OnceLockExt},
    utils,
};
use freedesktop_desktop_entry::DesktopEntry;
use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};

static FLATPAK_MANIFEST_IN: &str = include_str!("../../flatpak/manifest.in");

fn main() -> Result<()> {
    println!("cargo:warning=Debug: build script is running!");
    config::init();
    let app_dirs = AppDirs::new();
    app_dirs.init()?;

    create_config_symlinks(&app_dirs);
    create_data_symlinks(&app_dirs);
    copy_dev_web_apps(&app_dirs);

    create_app_desktop_file()?;
    create_app_icon()?;
    create_app_metainfo_file()?;
    update_flatpak_manifest()?;

    Ok(())
}

fn create_config_symlinks(app_dirs: &AppDirs) {
    let config_path = build_dev_config_path();
    let _ = utils::files::create_symlink(&config_path, &app_dirs.config());
}

fn create_data_symlinks(app_dirs: &AppDirs) {
    let data_path = build_dev_data_path();

    let _ = utils::files::create_symlink(&data_path, &app_dirs.data());
    let _ = utils::files::create_symlink(&data_path.join("applications"), &app_dirs.applications());
}

fn copy_dev_web_apps(app_dirs: &AppDirs) {
    let dev_desktop_files = build_dev_assets_path().join("desktop-files");
    let user_applications_dir = app_dirs.applications();

    for desktop_file in utils::files::get_entries_in_dir(&dev_desktop_files).unwrap() {
        fs::copy(
            desktop_file.path(),
            user_applications_dir.join(desktop_file.file_name()),
        )
        .unwrap();
    }
}

fn create_app_desktop_file() -> Result<()> {
    let desktop_file = assets::get_desktop_file();
    let app_id = config::APP_ID.get_value();
    let app_name = config::APP_NAME.get_value();
    let app_name_short = config::APP_NAME_SHORT.get_value();
    let extension = "desktop";
    let file_name = format!("{app_id}.{extension}");
    let save_dir = build_assets_path().join("desktop");
    let save_path = save_dir.join(file_name);

    let mut base_desktop_file =
        DesktopEntry::from_str(&save_path, desktop_file, None::<&[String]>)?;

    base_desktop_file.add_desktop_entry("Name".to_string(), app_name.clone());
    base_desktop_file.add_desktop_entry("Icon".to_string(), app_id.clone());
    base_desktop_file.add_desktop_entry("StartupWMClass".to_string(), app_id.clone());
    base_desktop_file.add_desktop_entry("Exec".to_string(), app_name_short.clone());

    if !save_dir.is_dir() {
        fs::create_dir_all(&save_dir)?;
    }

    fs::write(&save_path, base_desktop_file.to_string())?;

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
    let app_id = config::APP_ID.get_value();
    let app_name = config::APP_NAME.get_value();
    // let app_name_hyphen = config::APP_NAME_HYPHEN.get_value();
    let developer = config::DEVELOPER.get_value();
    let developer_id = &developer.to_lowercase();
    let app_summary = config::APP_SUMMARY.get_value();
    let app_description = config::APP_DESCRIPTION.get_value();
    let license = config::LICENSE.get_value();
    let repository = config::REPOSITORY.get_value();
    let assets_path = build_assets_path();

    // Change to this when name is final and repo name has changed
    // let screenshot_base_url = &format!(
    //     "https://raw.githubusercontent.com/{developer}/{app_name_hyphen}/refs/heads/main/assets/screenshots"
    // );
    let screenshot_base_url = &format!(
        "https://raw.githubusercontent.com/{developer}/gnome-web-apps/refs/heads/main/assets/screenshots"
    );
    let screenshots = utils::files::get_entries_in_dir(&assets_path.join("screenshots"))?
        .iter()
        .map(|file| {
            format!(
                "<image>{screenshot_base_url}/{}</image>\n",
                file.file_name().display()
            )
        })
        .collect::<Vec<String>>()
        .join("\n");

    let mut meta_data = assets::get_meta_info().to_string();
    meta_data = meta_data.replace("%{app_id}", app_id);
    meta_data = meta_data.replace("%{app_name}", app_name);
    meta_data = meta_data.replace("%{developer}", developer);
    meta_data = meta_data.replace("%{developer_id}", developer_id);
    meta_data = meta_data.replace("%{app_summary}", app_summary);
    meta_data = meta_data.replace("%{app_description}", app_description);
    meta_data = meta_data.replace("%{license}", license);
    meta_data = meta_data.replace("%{repository}", repository);
    meta_data = meta_data.replace("%{screenshots}", &screenshots);

    let save_dir = assets_path.join("desktop");
    let save_path = save_dir.join(format!("{app_id}.metainfo.xml"));

    if !save_dir.is_dir() {
        fs::create_dir_all(&save_dir)?;
    }
    fs::write(&save_path, meta_data)?;

    match Command::new("appstreamcli")
        .arg("validate")
        .arg("--no-net")
        .arg(save_path.as_os_str())
        .output()
    {
        Err(error) => bail!(error),
        Ok(output) => {
            if !output.status.success() {
                bail!("Metainfo file does not validate!")
            }
        }
    }

    Ok(())
}

fn update_flatpak_manifest() -> Result<()> {
    let app_id = config::APP_ID.get_value();
    let app_name = config::APP_NAME.get_value();
    let app_name_dense = config::APP_NAME_DENSE.get_value();
    let app_name_short = config::APP_NAME_SHORT.get_value();
    let app_name_hyphen = config::APP_NAME_HYPHEN.get_value();

    let mut manifest = FLATPAK_MANIFEST_IN.to_string();
    manifest = manifest.replace("%{app_id}", app_id);
    manifest = manifest.replace("%{app_name}", app_name);
    manifest = manifest.replace("%{app_name_dense}", app_name_dense);
    manifest = manifest.replace("%{app_name_short}", app_name_short);
    manifest = manifest.replace("%{app_name_hyphen}", app_name_hyphen);

    let save_path = Path::new("..")
        .join("..")
        .join("flatpak")
        .join("manifest.yml");
    fs::write(save_path, manifest)?;

    Ok(())
}

fn project_path() -> PathBuf {
    Path::new("").join("..").join("..").canonicalize().unwrap()
}

fn build_assets_path() -> PathBuf {
    project_path().join("assets")
}

fn build_dev_config_path() -> PathBuf {
    project_path().join("dev-config")
}

fn build_dev_data_path() -> PathBuf {
    project_path().join("dev-data")
}

fn build_dev_assets_path() -> PathBuf {
    project_path().join("dev-assets")
}
