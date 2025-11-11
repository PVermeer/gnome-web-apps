use super::App;
use gtk::Image;
use log::{debug, error, info};
use std::{
    cell::RefCell,
    fs,
    path::{Path, PathBuf},
    process::Command,
    rc::Rc,
};

pub enum Installation {
    Flatpak,
    System,
}
impl Installation {
    pub fn get_name(&self) -> String {
        match self {
            Self::Flatpak => String::from("Flatpak"),
            Self::System => String::from("System"),
        }
    }
}

#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
struct BrowserConfig {
    name: String,
    icon_name: Option<String>,
    flatpak: Option<String>,
    system_bin: Option<String>,
    #[serde(default)]
    can_isolate: bool,
}
pub struct Browser {
    pub name: String,
    pub icon: Image,
    pub installation: Installation,
    pub can_isolate: bool,
}
impl Browser {
    pub fn new(
        name: &str,
        icon_name: Option<&str>,
        installation: Installation,
        can_isolate: bool,
    ) -> Self {
        let mut icon = if let Some(icon_name) = icon_name {
            Image::from_icon_name(icon_name)
        } else {
            Image::from_icon_name("web-browser-symbolic")
        };
        if icon.uses_fallback() {
            icon = Image::from_icon_name("web-browser-symbolic");
        }

        Self {
            name: name.to_string(),
            icon,
            installation,
            can_isolate,
        }
    }
}

pub struct BrowserConfigs {
    pub flatpak: RefCell<Vec<Browser>>,
    pub system: RefCell<Vec<Browser>>,
}
impl BrowserConfigs {
    pub fn new() -> Self {
        Self {
            flatpak: RefCell::new(Vec::new()),
            system: RefCell::new(Vec::new()),
        }
    }

    pub fn init(&self, app: &Rc<App>) {
        self.set_browsers_from_files(app);
    }

    fn is_installed_flatpak(flatpak: &str) -> bool {
        let command = "flatpak";
        let arguments = &["info", flatpak];

        let output = Command::new(command).args(arguments).output();

        match output {
            Err(error) => {
                error!("Could not run command '{command} {arguments:?}'. Error: {error}");
                false
            }
            Ok(response) => response.status.success(),
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

    fn set_browsers_from_files(&self, app: &Rc<App>) {
        let browser_configs = Self::get_browsers_from_files(app);
        let mut flatpak_browsers = self.flatpak.borrow_mut();
        let mut system_browsers = self.system.borrow_mut();

        for (browser_config, file_name) in browser_configs {
            let name = browser_config.name.clone();
            let can_isolate = browser_config.can_isolate;

            if let Some(flatpak) = &browser_config.flatpak {
                if Self::is_installed_flatpak(flatpak) {
                    info!("Found flatpak browser '{flatpak}' from config '{file_name}'");

                    flatpak_browsers.push(Browser::new(
                        &name,
                        browser_config.icon_name.as_deref(),
                        Installation::Flatpak,
                        can_isolate,
                    ));
                } else {
                    info!("Flatpak browser '{flatpak}' from '{file_name}' is not installed");
                }
            }

            if let Some(system_bin) = &browser_config.system_bin {
                if Self::is_installed_system(system_bin) {
                    info!("Found system browser '{system_bin}' from config '{file_name}'");

                    system_browsers.push(Browser::new(
                        &name,
                        browser_config.icon_name.as_deref(),
                        Installation::System,
                        can_isolate,
                    ));
                } else {
                    info!("System browser '{system_bin}' from '{file_name}' is not installed");
                }
            }
        }
    }

    fn get_browsers_from_files(app: &Rc<App>) -> Vec<(BrowserConfig, String)> {
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
            browser_configs.push((browser, file_name.to_string()));

            info!("Loaded browser config: '{file_name}'");
        }

        browser_configs
    }
}
