use super::App;
use gtk::Image;
use log::{debug, error, info};
use std::fmt::Write as _;
use std::{
    cell::RefCell,
    fs,
    path::{Path, PathBuf},
    process::Command,
    rc::Rc,
};

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
}

#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BrowserConfig {
    name: String,
    icon_name: Option<String>,
    flatpak: Option<String>,
    system_bin: Option<String>,
    #[serde(default)]
    can_isolate: bool,
}
pub struct Browser {
    pub id: String,
    pub name: String,
    pub installation: Installation,
    pub can_isolate: bool,
    pub flatpak_id: Option<String>,
    pub executable: Option<String>,
    icon_name: String,
}
impl Browser {
    const FALLBACK_IMAGE: &str = "web-browser-symbolic";

    pub fn new(browser_config: &BrowserConfig, installation: Installation) -> Self {
        let icon_name = if let Some(icon_name) = browser_config.icon_name.clone() {
            icon_name
        } else {
            Self::FALLBACK_IMAGE.to_string()
        };

        let name = browser_config.name.clone();
        let can_isolate = browser_config.can_isolate;
        let flatpak_id = browser_config.flatpak.clone();
        let executable = browser_config.system_bin.clone();

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
            icon_name,
        }
    }

    pub fn is_flatpak(&self) -> bool {
        matches!(self.installation, Installation::Flatpak(_))
    }

    pub fn is_system(&self) -> bool {
        matches!(self.installation, Installation::System)
    }

    pub fn get_icon(&self) -> Image {
        let mut image = Image::from_icon_name(&self.icon_name);
        if image.uses_fallback() {
            image = Image::from_icon_name(Self::FALLBACK_IMAGE);
        }

        image
    }
}

pub struct BrowserConfigs {
    all_browsers: RefCell<Vec<Rc<Browser>>>,
}
impl BrowserConfigs {
    pub fn new() -> Self {
        Self {
            all_browsers: RefCell::new(Vec::new()),
        }
    }

    pub fn init(&self, app: &Rc<App>) {
        self.set_browsers_from_files(app);
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

    fn set_browsers_from_files(&self, app: &Rc<App>) {
        let browser_configs = Self::get_browsers_from_files(app);
        let mut all_browser_borrow = self.all_browsers.borrow_mut();

        for (browser_config, file_name) in browser_configs {
            if let Some(flatpak) = &browser_config.flatpak {
                if let Some(installation) = Self::is_installed_flatpak(flatpak) {
                    info!("Found flatpak browser '{flatpak}' from config '{file_name}'");

                    let browser = Rc::new(Browser::new(
                        &browser_config.clone(),
                        Installation::Flatpak(installation),
                    ));

                    all_browser_borrow.push(browser);
                } else {
                    info!("Flatpak browser '{flatpak}' from '{file_name}' is not installed");
                }
            }

            if let Some(system_bin) = &browser_config.system_bin {
                if Self::is_installed_system(system_bin) {
                    info!("Found system browser '{system_bin}' from config '{file_name}'");

                    let browser =
                        Rc::new(Browser::new(&browser_config.clone(), Installation::System));

                    all_browser_borrow.push(browser);
                } else {
                    info!("System browser '{system_bin}' from '{file_name}' is not installed");
                }
            }
        }
    }

    fn is_installed_flatpak(flatpak: &str) -> Option<FlatpakInstallation> {
        let command = "flatpak";
        let arguments = &["info", flatpak];

        let output = Command::new(command).args(arguments).output();

        match output {
            Err(error) => {
                error!("Could not run command '{command} {arguments:?}'. Error: {error}");
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
                    debug!("'{flatpak}' is user installed");
                    return Some(FlatpakInstallation::User);
                }
                if installation == FlatpakInstallation::System.to_string() {
                    debug!("'{flatpak}' is system installed");
                    return Some(FlatpakInstallation::System);
                }

                debug!("Could not determine installation type for '{flatpak}'");
                None
            }
        }
    }

    fn is_installed_system(system_bin: &str) -> bool {
        let command = "which";
        let arguments = &[system_bin];

        let output = Command::new(command).args(arguments).output();

        match output {
            Err(error) => {
                error!("Could not run command '{command} {arguments:?}'. Error: {error}");
                false
            }
            Ok(response) => response.status.success(),
        }
    }

    fn get_browsers_from_files(app: &Rc<App>) -> Vec<(Rc<BrowserConfig>, String)> {
        debug!("Loading browsers configs");

        let browsers_dir = Path::new("browsers");
        let mut browser_files: Vec<PathBuf> = app.dirs.find_data_files(browsers_dir).collect();
        let mut browser_configs = Vec::new();

        if cfg!(debug_assertions) {
            let dev_browser_dir = Path::new("./assets/").join(browsers_dir);
            debug!(
                "Loading dev browser files from: '{}'",
                dev_browser_dir.to_string_lossy()
            );

            let Ok(dev_browser_files) = fs::read_dir(&dev_browser_dir) else {
                return browser_configs;
            };

            for file in dev_browser_files {
                let Ok(dir_entry) = file else {
                    continue;
                };
                browser_files.push(dir_entry.path());
            }
        }

        for file_path in browser_files {
            debug!("Loading browser configs");

            let file_name = file_path.file_name().unwrap_or_default().to_string_lossy();
            let extension = file_path.extension().unwrap_or_default().to_string_lossy();

            if extension != "yml" && extension != "yaml" {
                debug!("Not a yml file: '{file_name}'");
                continue;
            }
            let Ok(file_string) = fs::read_to_string(&file_path) else {
                debug!("Failed to read to string: '{file_name}'");
                continue;
            };
            let browser: BrowserConfig = match serde_yaml::from_str(&file_string) {
                Ok(result) => result,
                Err(error) => {
                    debug!("Failed to parse yml: '{file_name}'. Error: '{error}'");
                    continue;
                }
            };
            browser_configs.push((Rc::new(browser), file_name.to_string()));

            info!("Loaded browser config: '{file_name}'");
        }

        browser_configs
    }
}
