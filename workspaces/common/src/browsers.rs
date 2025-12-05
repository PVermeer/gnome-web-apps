use crate::utils;
use crate::{
    app_dirs::AppDirs,
    config::{self, OnceLockExt},
};
use anyhow::{Context, Result, bail};
use freedesktop_desktop_entry::DesktopEntry;
use gtk::{IconTheme, Image};
use std::{cell::RefCell, collections::HashSet, fs, path::Path, process::Command, rc::Rc};
use std::{fmt::Write as _, path::PathBuf};
use tracing::{debug, error, info};

#[derive(PartialEq)]
pub enum FlatpakInstallation {
    System,
    User,
}
impl std::fmt::Display for FlatpakInstallation {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::System => write!(f, "system"),
            Self::User => write!(f, "user"),
        }
    }
}
#[derive(PartialEq)]
pub enum Installation {
    Flatpak(FlatpakInstallation),
    System,
    None,
}

#[derive(PartialEq)]
pub enum Base {
    Chromium,
    Firefox,
    None,
}
impl Base {
    fn from_string(string: &str) -> Self {
        match string {
            "chromium" => Self::Chromium,
            "firefox" => Self::Firefox,
            _ => Self::None,
        }
    }
}

#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BrowserYaml {
    name: String,
    flatpak: Option<String>,
    system_bin: Option<String>,
    #[serde(default)]
    can_isolate: bool,
    desktop_file_name_prefix: String,
    base: String,
}

struct BrowserConfig {
    config: BrowserYaml,
    file_name: String,
    desktop_file: DesktopEntry,
}

pub struct Browser {
    pub id: String,
    pub name: String,
    pub installation: Installation,
    pub can_isolate: bool,
    pub flatpak_id: Option<String>,
    pub executable: Option<String>,
    pub desktop_file: DesktopEntry,
    pub desktop_file_name_prefix: String,
    pub base: Base,
    configs: Rc<BrowserConfigs>,
    icon_theme: Rc<IconTheme>,
    icon_names: HashSet<String>,
    app_dirs: Rc<AppDirs>,
}
impl Browser {
    const FALLBACK_IMAGE: &str = "web-browser-symbolic";

    fn new(
        browser_config: &BrowserConfig,
        installation: Installation,
        browser_configs: &Rc<BrowserConfigs>,
        icon_theme: &Rc<IconTheme>,
        app_dirs: &Rc<AppDirs>,
    ) -> Self {
        let icon_names = Self::get_icon_names_from_config(browser_config);
        let name = browser_config.config.name.clone();
        let can_isolate = browser_config.config.can_isolate;
        let flatpak_id = browser_config.config.flatpak.clone();
        let executable = browser_config.config.system_bin.clone();
        let desktop_file = browser_config.desktop_file.clone();
        let desktop_file_name_prefix = browser_config.config.desktop_file_name_prefix.clone();
        let base = Base::from_string(&browser_config.config.base);

        let id = if matches!(installation, Installation::Flatpak(_)) {
            flatpak_id.clone().unwrap()
        } else if matches!(installation, Installation::System) {
            executable.clone().unwrap()
        } else {
            panic!("Could not create id for Browser")
        };

        Self {
            id,
            name,
            installation,
            can_isolate,
            flatpak_id,
            executable,
            desktop_file,
            desktop_file_name_prefix,
            configs: browser_configs.clone(),
            icon_names,
            base,
            icon_theme: icon_theme.clone(),
            app_dirs: app_dirs.clone(),
        }
    }

    pub fn is_flatpak(&self) -> bool {
        matches!(self.installation, Installation::Flatpak(_))
    }

    pub fn is_system(&self) -> bool {
        matches!(self.installation, Installation::System)
    }

    pub fn get_name_with_installation(&self) -> String {
        let mut txt = String::new();
        let _ = write!(txt, "{}", self.name);

        match self.installation {
            Installation::Flatpak(_) => {
                let _ = write!(txt, " (Flatpak)");
            }
            Installation::System => {
                let _ = write!(txt, " (System)");
            }
            Installation::None => {}
        }

        txt
    }

    pub fn get_command(&self) -> Result<String> {
        match &self.installation {
            Installation::Flatpak(installation) => {
                let Some(flatpak_id) = &self.flatpak_id else {
                    bail!("No flatpak id with flatpak installation")
                };
                match installation {
                    FlatpakInstallation::User => Ok(format!("flatpak run --user {flatpak_id}")),
                    FlatpakInstallation::System => Ok(format!("flatpak run --system {flatpak_id}")),
                }
            }
            Installation::System => {
                let Some(executable) = &self.executable else {
                    bail!("No executable with system installation")
                };
                Ok(executable.clone())
            }
            Installation::None => bail!("Browser is not installed"),
        }
    }

    pub fn get_icon(&self) -> Image {
        for icon in &self.icon_names {
            if !self.icon_theme.has_icon(icon) {
                continue;
            }
            let image = Image::from_icon_name(icon);
            if image.uses_fallback() {
                continue;
            }
            return image;
        }

        Image::from_icon_name(Self::FALLBACK_IMAGE)
    }

    pub fn get_run_command(&self) -> Result<String> {
        match self.installation {
            Installation::Flatpak(_) => {
                let Some(flatpak_id) = self.flatpak_id.clone() else {
                    bail!("No flatpak id on flatpak installation???")
                };

                let command = format!("flatpak run {flatpak_id}");
                Ok(command)
            }
            Installation::System => {
                let Some(executable) = self.executable.clone() else {
                    bail!("No flatpak id on flatpak installation???")
                };
                Ok(executable)
            }
            Installation::None => bail!("No installation type on 'Browser'"),
        }
    }

    pub fn get_profile_path(&self, app_id: &str) -> Result<String> {
        if !self.can_isolate {
            bail!("Browser cannot isolate")
        }

        // Save in own app
        let app_profile_path = || -> Result<PathBuf> {
            let path = self.app_dirs.profiles().join(&self.id).join(app_id);
            if !path.is_dir() {
                debug!(
                    path = path.to_string_lossy().to_string(),
                    "Creating profile path"
                );
                fs::create_dir_all(&path)
                    .context(format!("Failed to create profile dir: {}", path.display()))?;
            }
            Ok(path)
        };

        // Save in browser own location (for sandboxes)
        let browser_profile_path = || -> Result<PathBuf> {
            let path = self
                .app_dirs
                .flatpak()
                .join(&self.id)
                .join("data")
                .join(config::APP_NAME_HYPHEN.get_value())
                .join("profiles")
                .join(app_id);
            if !path.is_dir() {
                debug!(
                    path = path.to_string_lossy().to_string(),
                    "Creating profile path"
                );
                fs::create_dir_all(&path)
                    .context(format!("Failed to create profile dir: {}", path.display()))?;
            }
            Ok(path)
        };

        let profile = match self.base {
            /*
               Firefox has a method to create profiles (-CreateProfile <name> and -P) but is poorly implemented.
               If firefox has never run it will set the created profile as default and
               never creates a default profile.
               Then there is --profile <path>, this works but will not create the path if it doesn't exists.
               So `--filesystem=~/.var/app:create` is needed to break in the sandbox to create the path if it doesn't exists.
               All a bit poorly implemented.

               Chromium based just created the provided profile path
            */
            Base::Chromium | Base::Firefox => {
                let profile_path = match self.installation {
                    Installation::Flatpak(_) => browser_profile_path()?,
                    Installation::System => app_profile_path()?,
                    Installation::None => bail!("No installation type on 'Browser'"),
                };
                profile_path.to_string_lossy().to_string()
            }

            Base::None => {
                bail!("No base browser on 'Browser'")
            }
        };

        if profile.is_empty() {
            bail!("Profile is an empty string")
        }

        Ok(profile)
    }

    pub fn copy_profile_config_to_profile_path(&self, app_id: &str) -> Result<()> {
        let profile_path = self.get_profile_path(app_id)?;

        let copy_options = fs_extra::dir::CopyOptions {
            overwrite: true,
            content_only: true,
            ..fs_extra::dir::CopyOptions::default()
        };

        let copy_profile_config = move |config_path: &PathBuf| -> Result<()> {
            if config_path.is_dir() {
                fs_extra::dir::copy(config_path, &profile_path, &copy_options)?;
            }
            Ok(())
        };

        match self.base {
            Base::Chromium => {
                let config_path = self.app_dirs.config().join("profiles").join("chromium");
                copy_profile_config(&config_path)
            }
            Base::Firefox => {
                let config_path = self.app_dirs.config().join("profiles").join("firefox");
                copy_profile_config(&config_path)
            }
            Base::None => Ok(()),
        }
    }

    pub fn get_index(&self) -> Option<usize> {
        self.configs.get_index(self)
    }

    fn get_icon_names_from_config(browser_config: &BrowserConfig) -> HashSet<String> {
        let mut icon_names = HashSet::new();

        if let Some(flatpak) = &browser_config.config.flatpak {
            icon_names.insert(flatpak.trim().to_string());
        }

        if let Some(bin) = &browser_config.config.system_bin {
            icon_names.insert(bin.trim().to_string());
        }

        icon_names.insert(browser_config.config.name.trim().to_string());

        icon_names
    }
}

pub struct BrowserConfigs {
    all_browsers: RefCell<Vec<Rc<Browser>>>,
    icon_theme: Rc<IconTheme>,
    app_dirs: Rc<AppDirs>,
}
impl BrowserConfigs {
    pub fn new(icon_theme: &Rc<IconTheme>, app_dirs: &Rc<AppDirs>) -> Rc<Self> {
        Rc::new(Self {
            all_browsers: RefCell::new(Vec::new()),
            icon_theme: icon_theme.clone(),
            app_dirs: app_dirs.clone(),
        })
    }

    pub fn init(self: &Rc<Self>) {
        let no_browser = self.get_no_browser();
        self.all_browsers.borrow_mut().push(Rc::new(no_browser));

        self.set_browsers_from_files();
    }

    pub fn get_all_browsers(&self) -> Vec<Rc<Browser>> {
        self.all_browsers.borrow().clone()
    }

    pub fn get_flatpak_browsers(&self) -> Vec<Rc<Browser>> {
        let all_browsers_borrow = self.all_browsers.borrow();
        all_browsers_borrow
            .iter()
            .filter(|browser| browser.is_flatpak())
            .cloned()
            .collect()
    }

    pub fn get_system_browsers(&self) -> Vec<Rc<Browser>> {
        let all_browsers_borrow = self.all_browsers.borrow();
        all_browsers_borrow
            .iter()
            .filter(|browser| browser.installation == Installation::System)
            .cloned()
            .collect()
    }

    pub fn get_by_id(&self, id: &str) -> Option<Rc<Browser>> {
        self.all_browsers
            .borrow()
            .iter()
            .find(|browser| browser.id == id)
            .cloned()
    }

    pub fn get_index(&self, browser: &Browser) -> Option<usize> {
        self.all_browsers
            .borrow()
            .iter()
            .position(|browser_iter| browser_iter.id == browser.id)
    }

    fn get_no_browser(self: &Rc<Self>) -> Browser {
        Browser {
            id: String::default(),
            name: "No browser".to_string(),
            installation: Installation::None,
            can_isolate: false,
            flatpak_id: None,
            executable: None,
            desktop_file: DesktopEntry::from_appid("No browser".to_string()),
            desktop_file_name_prefix: String::default(),
            configs: self.clone(),
            icon_names: HashSet::from(["dialog-warning-symbolic".to_string()]),
            base: Base::None,
            icon_theme: self.icon_theme.clone(),
            app_dirs: self.app_dirs.clone(),
        }
    }

    fn set_browsers_from_files(self: &Rc<Self>) {
        let browser_configs = self.get_browsers_from_files();
        let mut all_browser_borrow = self.all_browsers.borrow_mut();

        for browser_config in browser_configs {
            if let Some(flatpak) = &browser_config.config.flatpak {
                if let Some(installation) = Self::is_installed_flatpak(flatpak) {
                    info!(
                        "Found flatpak browser '{flatpak} ({installation})' for config '{}'",
                        browser_config.file_name
                    );

                    let browser = Rc::new(Browser::new(
                        &browser_config,
                        Installation::Flatpak(installation),
                        self,
                        &self.icon_theme,
                        &self.app_dirs,
                    ));

                    if utils::env::is_flatpak_container()
                        && let Some(icon_search_path) = Self::get_icon_search_path_flatpak(flatpak)
                    {
                        self.add_icon_search_path(&icon_search_path);
                    }

                    all_browser_borrow.push(browser);
                } else {
                    debug!(
                        "Flatpak browser '{flatpak}' for '{}' is not installed",
                        browser_config.file_name
                    );
                }
            }

            if let Some(system_bin) = &browser_config.config.system_bin {
                if Self::is_installed_system(system_bin) {
                    info!(
                        "Found system browser '{system_bin}' for config '{}'",
                        browser_config.file_name
                    );

                    let browser = Rc::new(Browser::new(
                        &browser_config,
                        Installation::System,
                        self,
                        &self.icon_theme,
                        &self.app_dirs,
                    ));

                    all_browser_borrow.push(browser);
                } else {
                    debug!(
                        "System browser '{system_bin}' for '{}' is not installed",
                        browser_config.file_name
                    );
                }
            }
        }
    }

    fn is_installed_flatpak(flatpak: &str) -> Option<FlatpakInstallation> {
        let (command, arguments, output) = if utils::env::is_flatpak_container() {
            let command = "flatpak-spawn";
            let arguments = Vec::from(["--host", "flatpak", "info", flatpak]);
            let output = Command::new(command).args(&arguments).output();
            (command, arguments, output)
        } else {
            let command = "flatpak";
            let arguments = Vec::from(["info", flatpak]);
            let output = Command::new(command).args(&arguments).output();
            (command, arguments, output)
        };

        match output {
            Err(error) => {
                error!("Could not run command '{command} {arguments:?}'. Error: {error:?}");
                None
            }
            Ok(response) => {
                if !response.status.success() {
                    return None;
                }
                let output_txt = String::from_utf8_lossy(&response.stdout);
                let installation_line = output_txt
                    .lines()
                    .find(|line| line.contains("Installation:"))?;
                let installation = installation_line.split("Installation: ").last()?;

                if installation == FlatpakInstallation::User.to_string() {
                    return Some(FlatpakInstallation::User);
                }
                if installation == FlatpakInstallation::System.to_string() {
                    return Some(FlatpakInstallation::System);
                }

                debug!("Could not determine installation type for '{flatpak}'");
                None
            }
        }
    }

    fn is_installed_system(system_bin: &str) -> bool {
        let (command, arguments, output) = if utils::env::is_flatpak_container() {
            let command = "flatpak-spawn";
            let arguments = Vec::from(["--host", "which", system_bin]);
            let output = Command::new(command).args(&arguments).output();
            (command, arguments, output)
        } else {
            let command = "which";
            let arguments = Vec::from([system_bin]);

            let output = Command::new(command).args(&arguments).output();
            (command, arguments, output)
        };

        match output {
            Err(error) => {
                error!("Could not run command '{command} {arguments:?}'. Error: {error:?}");
                false
            }
            Ok(response) => response.status.success(),
        }
    }

    fn get_icon_search_path_flatpak(flatpak: &str) -> Option<PathBuf> {
        if !utils::env::is_flatpak_container() {
            error!("Don't need to get icon search path when not in flatpak container");
            return None;
        }

        let command = "flatpak-spawn";
        let arguments = &["--host", "flatpak", "info", "--show-location", flatpak];
        let output = Command::new(command).args(arguments).output();

        match output {
            Err(error) => {
                error!("Could not run command '{command} {arguments:?}'. Error: {error:?}");
                None
            }
            Ok(response) => {
                if !response.status.success() {
                    error!("Could not get icon search path for: {flatpak}");
                    return None;
                }
                let output_txt = String::from_utf8_lossy(&response.stdout).trim().to_string();

                let path = Path::new(&output_txt)
                    .join("export")
                    .join("share")
                    .join("icons");

                if !path.is_dir() {
                    error!("Invalid icon path for '{flatpak}': {}", path.display());
                    return None;
                }

                Some(path)
            }
        }
    }

    fn get_browsers_from_files(&self) -> Vec<Rc<BrowserConfig>> {
        debug!("Loading browsers config files");

        let mut browser_configs = Vec::new();
        let browser_config_files =
            utils::files::get_entries_in_dir(&self.app_dirs.browser_configs()).unwrap_or_default();

        for file in &browser_config_files {
            let file_name = file.file_name().to_string_lossy().to_string();
            let file_path = file.path();

            let extension = file_path.extension().unwrap_or_default().to_string_lossy();
            debug!("Loading browser config: '{file_name}'");

            if extension != "yml" && extension != "yaml" {
                debug!("Not a yml file: '{file_name}'");
                continue;
            }

            let Ok(file_string) = fs::read_to_string(&file_path) else {
                error!("Failed to read to string: '{file_name}'");
                continue;
            };
            let browser: BrowserYaml = match serde_yaml::from_str(&file_string) {
                Ok(result) => result,
                Err(error) => {
                    error!("Failed to parse yml: '{file_name}'. Error: '{error:?}'");
                    continue;
                }
            };

            let desktop_file = match (|| -> Result<DesktopEntry> {
                let desktop_file_path = self
                    .app_dirs
                    .browser_desktop_files()
                    .join(
                        file_path
                            .file_stem()
                            .context("Could not get the file stem")?,
                    )
                    .with_extension("desktop");
                let desktop_file = DesktopEntry::from_path(&desktop_file_path, None::<&[String]>)?;
                Ok(desktop_file)
            })() {
                Ok(result) => result,
                Err(error) => {
                    error!("Failed to parse .desktop file for: '{file_name}'. Error: '{error:?}'");
                    continue;
                }
            };

            let browser_config = BrowserConfig {
                config: browser,
                desktop_file,
                file_name,
            };
            browser_configs.push(Rc::new(browser_config));
        }

        browser_configs
    }

    pub fn add_icon_search_path(self: &Rc<Self>, path: &Path) {
        if !path.is_dir() {
            debug!("Not a valid icon path: {}", path.display());
            return;
        }

        debug!("Adding icon path to icon theme: {}", path.display());
        self.icon_theme.add_search_path(path);
    }
}
