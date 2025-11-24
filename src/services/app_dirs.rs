use crate::services::config;
use anyhow::{Context, Result};
use gtk::glib;
use std::{
    cell::OnceCell,
    fs, os,
    path::{Path, PathBuf},
    rc::Rc,
};
use tracing::debug;

#[derive(Default)]
pub struct AppDirs {
    home: OnceCell<PathBuf>,
    data: OnceCell<PathBuf>,
    config: OnceCell<PathBuf>,
    user_applications: OnceCell<PathBuf>,
    profiles: OnceCell<PathBuf>,
    icons: OnceCell<PathBuf>,
    browser_configs: OnceCell<PathBuf>,
    browser_desktop_files: OnceCell<PathBuf>,
    flatpak: OnceCell<PathBuf>,
}
impl AppDirs {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    pub fn init(&self) -> Result<()> {
        let home = glib::home_dir();
        let user_data = glib::user_data_dir().join(config::APP_NAME_PATH);
        let user_config = glib::user_config_dir().join(config::APP_NAME_PATH);

        let _ = self.home.set(home);
        let _ = self.data.set(user_data);
        let _ = self.config.set(user_config);

        let applications = self.build_applications_path()?;
        let profiles = self.build_profiles_path()?;
        let icons = self.build_icons_path()?;
        let browser_configs = self.build_browser_configs_path()?;
        let browser_desktop_files = self.build_browser_desktop_files_path()?;
        let flatpak = self.build_flatpak_path();

        let _ = self.user_applications.set(applications);
        let _ = self.profiles.set(profiles);
        let _ = self.icons.set(icons);
        let _ = self.browser_configs.set(browser_configs);
        let _ = self.browser_desktop_files.set(browser_desktop_files);
        let _ = self.flatpak.set(flatpak);

        Ok(())
    }

    pub fn home(&self) -> PathBuf {
        self.home.get().unwrap().clone()
    }

    pub fn config(&self) -> PathBuf {
        self.config.get().unwrap().clone()
    }

    pub fn data(&self) -> PathBuf {
        self.data.get().unwrap().clone()
    }

    pub fn applications(&self) -> PathBuf {
        self.user_applications.get().unwrap().clone()
    }

    pub fn profiles(&self) -> PathBuf {
        self.profiles.get().unwrap().clone()
    }

    pub fn icons(&self) -> PathBuf {
        self.icons.get().unwrap().clone()
    }

    pub fn browser_configs(&self) -> PathBuf {
        self.browser_configs.get().unwrap().clone()
    }

    pub fn browser_desktop_files(&self) -> PathBuf {
        self.browser_desktop_files.get().unwrap().clone()
    }

    pub fn flatpak(&self) -> PathBuf {
        self.flatpak.get().unwrap().clone()
    }

    fn build_applications_path(&self) -> Result<PathBuf> {
        let applications_dir_name = "applications";
        let mut system_applications_path = glib::user_data_dir().join("applications");
        let mut app_applications_path = self.data().join(applications_dir_name);

        if cfg!(debug_assertions) {
            system_applications_path = std::path::absolute(Path::new("./dev-assets/desktop-files"))
                .context("Dev-only: system_applications path to absolute failed")?;
            app_applications_path = std::path::absolute(Path::new("./dev-data/applications"))
                .context("Dev-only: app_applications path to absolute failed")?;
        }

        debug!(
            "Using system applications path: {}",
            system_applications_path.display()
        );
        debug!(
            "Using app applications path: {}",
            app_applications_path.display()
        );

        if !app_applications_path.is_symlink() {
            os::unix::fs::symlink(&system_applications_path, &app_applications_path)
                .context("Could not symlink system applications dir to data dir")?;
        }

        Ok(app_applications_path)
    }

    fn build_profiles_path(&self) -> Result<PathBuf> {
        let profiles_dir_name = "profiles";
        let mut profiles_path = self.data().join(profiles_dir_name);

        if cfg!(debug_assertions) {
            profiles_path = Path::new("dev-data").join(profiles_dir_name);
        }

        debug!("Using profile path: {}", profiles_path.display());

        if !profiles_path.is_dir() {
            fs::create_dir_all(&profiles_path).context("Could not create profiles dir")?;
        }

        Ok(profiles_path)
    }

    fn build_icons_path(&self) -> Result<PathBuf> {
        let icons_dir_name = "icons";
        let mut icons_path = self.data().join(icons_dir_name);

        if cfg!(debug_assertions) {
            icons_path = Path::new("dev-data").join(icons_dir_name);
        }

        debug!("Using icons path: {}", icons_path.display());

        if !icons_path.is_dir() {
            fs::create_dir_all(&icons_path).context("Could not create icons dir")?;
        }

        Ok(icons_path)
    }

    fn build_browser_configs_path(&self) -> Result<PathBuf> {
        let browsers_dir_name = "browsers";
        let mut browser_configs_path = self.config().join(browsers_dir_name);

        if cfg!(debug_assertions) {
            browser_configs_path = Path::new("assets").join(browsers_dir_name);
        }

        debug!("Using browsers path: {}", browser_configs_path.display());

        if !browser_configs_path.is_dir() {
            fs::create_dir_all(&browser_configs_path).context("Could not create browsers dir")?;
        }

        Ok(browser_configs_path)
    }

    fn build_browser_desktop_files_path(&self) -> Result<PathBuf> {
        let browsers_desktop_files_dir_name = "desktop-files";
        let mut browser_desktop_files_path = self.config().join(browsers_desktop_files_dir_name);

        if cfg!(debug_assertions) {
            browser_desktop_files_path = Path::new("assets").join(browsers_desktop_files_dir_name);
        }

        debug!(
            "Using browser desktop-files path: {}",
            browser_desktop_files_path.display()
        );

        if !browser_desktop_files_path.is_dir() {
            fs::create_dir_all(&browser_desktop_files_path)
                .context("Could not create browser desktop-files dir")?;
        }

        Ok(browser_desktop_files_path)
    }

    fn build_flatpak_path(&self) -> PathBuf {
        let flatpak_path = self.home().join(".var").join("app");

        debug!("Using flatpak path: {}", flatpak_path.display());

        flatpak_path
    }
}
