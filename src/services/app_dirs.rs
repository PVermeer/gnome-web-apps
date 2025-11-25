use crate::services::{config, utils};
use anyhow::{Context, Result};
use gtk::glib;
use std::{
    cell::OnceCell,
    fs,
    path::{Path, PathBuf},
    rc::Rc,
};
use tracing::debug;

#[derive(Default)]
pub struct AppDirs {
    home: OnceCell<PathBuf>,
    data: OnceCell<PathBuf>,
    config: OnceCell<PathBuf>,
    system_data: OnceCell<Vec<PathBuf>>,
    system_icons: OnceCell<Vec<PathBuf>>,
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
        let system_data = glib::system_data_dirs();

        let _ = self.home.set(home);
        let _ = self.data.set(user_data);
        let _ = self.config.set(user_config);
        let _ = self.system_data.set(system_data);

        let system_icons = self.build_system_icon_paths();
        let applications = Self::build_applications_path()?;
        let profiles = self.build_profiles_path()?;
        let icons = self.build_icons_path()?;
        let browser_configs = self.build_browser_configs_path()?;
        let browser_desktop_files = self.build_browser_desktop_files_path()?;
        let flatpak = self.build_flatpak_path();

        let _ = self.system_icons.set(system_icons);
        let _ = self.user_applications.set(applications);
        let _ = self.profiles.set(profiles);
        let _ = self.icons.set(icons);
        let _ = self.browser_configs.set(browser_configs);
        let _ = self.browser_desktop_files.set(browser_desktop_files);
        let _ = self.flatpak.set(flatpak);

        if cfg!(debug_assertions) {
            utils::files::create_symlink(Path::new("dev-config"), &self.config());
            utils::files::create_symlink(Path::new("dev-data"), &self.data());
            utils::files::create_symlink(
                &Path::new("dev-data").join("applications"),
                &self.applications(),
            );

            for file in
                utils::files::get_entries_in_dir(&Path::new("dev-assets").join("desktop-files"))
                    .unwrap_or_default()
            {
                utils::files::create_symlink(
                    &self.applications().join(file.file_name()),
                    &file.path(),
                );
            }
        }

        Ok(())
    }

    pub fn home(&self) -> PathBuf {
        self.home.get().unwrap().clone()
    }

    pub fn data(&self) -> PathBuf {
        self.data.get().unwrap().clone()
    }

    pub fn config(&self) -> PathBuf {
        self.config.get().unwrap().clone()
    }

    pub fn system_data(&self) -> Vec<PathBuf> {
        self.system_data.get().unwrap().clone()
    }

    pub fn system_icons(&self) -> Vec<PathBuf> {
        self.system_icons.get().unwrap().clone()
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

    fn build_system_icon_paths(&self) -> Vec<PathBuf> {
        let icons_dir_name = "icons";
        self.system_data()
            .iter()
            .map(|path| path.join(icons_dir_name))
            .filter(|path| path.is_dir())
            .collect()
    }

    fn build_applications_path() -> Result<PathBuf> {
        let user_applications_path = glib::user_data_dir().join("applications");

        debug!(
            "Using system applications path: {}",
            user_applications_path.display()
        );

        if !user_applications_path.is_dir() {
            fs::create_dir_all(&user_applications_path).context(format!(
                "Could not create user applications dir: {}",
                user_applications_path.display()
            ))?;
        }

        Ok(user_applications_path)
    }

    fn build_profiles_path(&self) -> Result<PathBuf> {
        let profiles_dir_name = "profiles";
        let profiles_path = self.data().join(profiles_dir_name);

        debug!("Using profile path: {}", profiles_path.display());

        if !profiles_path.is_dir() {
            fs::create_dir_all(&profiles_path).context(format!(
                "Could not create profiles dir: {}",
                profiles_path.display()
            ))?;
        }

        Ok(profiles_path)
    }

    fn build_icons_path(&self) -> Result<PathBuf> {
        let icons_dir_name = "icons";
        let icons_path = self.data().join(icons_dir_name);

        debug!("Using icons path: {}", icons_path.display());

        if !icons_path.is_dir() {
            fs::create_dir_all(&icons_path).context(format!(
                "Could not create icons dir: {}",
                icons_path.display()
            ))?;
        }

        Ok(icons_path)
    }

    fn build_browser_configs_path(&self) -> Result<PathBuf> {
        let browsers_dir_name = "browsers";
        let browser_configs_path = self.config().join(browsers_dir_name);

        debug!("Using browsers path: {}", browser_configs_path.display());

        if !browser_configs_path.is_dir() {
            fs::create_dir_all(&browser_configs_path).context(format!(
                "Could not create browsers dir: {}",
                browser_configs_path.display()
            ))?;
        }

        Ok(browser_configs_path)
    }

    fn build_browser_desktop_files_path(&self) -> Result<PathBuf> {
        let browsers_desktop_files_dir_name = "desktop-files";
        let browser_desktop_files_path = self.config().join(browsers_desktop_files_dir_name);

        debug!(
            "Using browser desktop-files path: {}",
            browser_desktop_files_path.display()
        );

        if !browser_desktop_files_path.is_dir() {
            fs::create_dir_all(&browser_desktop_files_path).context(format!(
                "Could not create browser desktop-files dir: {}",
                browser_desktop_files_path.display()
            ))?;
        }

        Ok(browser_desktop_files_path)
    }

    fn build_flatpak_path(&self) -> PathBuf {
        let flatpak_path = self.home().join(".var").join("app");

        debug!("Using flatpak path: {}", flatpak_path.display());

        flatpak_path
    }
}
