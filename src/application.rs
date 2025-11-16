mod pages;
mod window;

use crate::{
    config,
    services::{browsers::BrowserConfigs, fetch::Fetch},
};
use anyhow::{Result, bail};
use freedesktop_desktop_entry::get_languages_from_env;
use log::debug;
use pages::{Page, Pages};
use std::{
    path::{Path, PathBuf},
    rc::Rc,
};
use window::AppWindow;
use xdg::BaseDirectories;

pub struct App {
    pub dirs: BaseDirectories,
    pub desktop_file_locales: Vec<String>,
    pub browsers_configs: BrowserConfigs,
    window: AppWindow,
    fetch: Fetch,
    pages: Pages,
}
impl App {
    pub fn new(adw_application: &libadwaita::Application) -> Rc<Self> {
        Rc::new({
            let window = AppWindow::new(adw_application);
            let app_dirs = BaseDirectories::with_prefix(config::APP_NAME_PATH);
            let fetch = Fetch::new();
            let pages = Pages::new();
            let browsers = BrowserConfigs::new();
            let desktop_file_locales = get_languages_from_env();

            Self {
                dirs: app_dirs,
                desktop_file_locales,
                browsers_configs: browsers,
                window,
                fetch,
                pages,
            }
        })
    }

    pub fn init(self: &Rc<Self>) {
        self.window.init(self);
        self.browsers_configs.init(self);
        self.pages.init(self);

        self.navigate(&Page::Home);
    }

    pub fn navigate(self: &Rc<Self>, page: &Page) {
        self.window.view.navigate(self, page);
    }

    pub fn get_applications_path(&self) -> Result<PathBuf> {
        if cfg!(debug_assertions) {
            let path = Path::new("./dev-assets/desktop-files").to_path_buf();
            debug!("Using dev applications path: {}", path.display());
            return Ok(path);
        }

        let Some(data_home_path) = self.dirs.data_home.as_ref() else {
            bail!("Could not get data home path")
        };

        let path = data_home_path.join("applications");
        debug!("Using applications path: {}", path.display());

        if !path.is_dir() {
            bail!("Could not get applications path");
        }

        Ok(path)
    }
}
