use super::App;
use log::{debug, info};
use std::{
    cell::RefCell,
    fs,
    path::{Path, PathBuf},
    rc::Rc,
};

#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
struct Browser {
    name: String,
    flatpak: Option<String>,
    command: Option<String>,
    #[serde(default)]
    can_isolate: bool,
}

pub struct BrowserConfigs {
    browsers: RefCell<Vec<Browser>>,
}
impl BrowserConfigs {
    pub fn new() -> Self {
        Self {
            browsers: RefCell::new(Vec::new()),
        }
    }

    pub fn init(&self, app: &Rc<App>) {
        self.set_browsers_from_files(app);
    }

    fn set_browsers_from_files(&self, app: &Rc<App>) {
        debug!("Loading browsers configs");

        let browsers_dir = Path::new("browsers");
        let mut browser_files: Vec<PathBuf> = app.dirs.find_data_files(browsers_dir).collect();
        let mut browsers = self.browsers.borrow_mut();

        if cfg!(debug_assertions) {
            let dev_browser_dir = Path::new("./assets/").join(browsers_dir);
            debug!(
                "Loading dev browser files from: '{}'",
                dev_browser_dir.to_string_lossy()
            );

            let Ok(dev_browser_files) = fs::read_dir(&dev_browser_dir) else {
                return;
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
            let browser: Browser = match serde_yaml::from_str(&file_string) {
                Ok(result) => result,
                Err(error) => {
                    debug!("Failed to parse yml: '{file_name}'. Error: '{error}'");
                    continue;
                }
            };
            browsers.push(browser);

            info!("Loaded browser config: '{file_name}'");
        }
    }
}
