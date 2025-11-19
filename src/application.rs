mod error_dialog;
mod pages;
mod window;

use crate::{
    config,
    services::{assets::Assets, browsers::BrowserConfigs, fetch::Fetch},
};
use anyhow::{Error, Result, bail};
use error_dialog::ErrorDialog;
use freedesktop_desktop_entry::get_languages_from_env;
use log::{debug, error};
use pages::{Page, Pages};
use std::{
    path::{Path, PathBuf},
    rc::Rc,
};
use window::AppWindow;
use xdg::BaseDirectories;

pub struct App {
    pub dirs: Rc<BaseDirectories>,
    pub desktop_file_locales: Vec<String>,
    pub browsers_configs: BrowserConfigs,
    pub error_dialog: ErrorDialog,
    adw_application: libadwaita::Application,
    window: AppWindow,
    fetch: Fetch,
    pages: Pages,
    assets: Assets,
}
impl App {
    pub fn new(adw_application: &libadwaita::Application) -> Rc<Self> {
        Rc::new({
            let window = AppWindow::new(adw_application);
            let app_dirs = Rc::new(BaseDirectories::with_prefix(config::APP_NAME_PATH));
            let fetch = Fetch::new();
            let pages = Pages::new();
            let browsers = BrowserConfigs::new();
            let desktop_file_locales = get_languages_from_env();
            let error_dialog = ErrorDialog::new();
            let assets = Assets::new(&app_dirs);

            Self {
                dirs: app_dirs,
                desktop_file_locales,
                browsers_configs: browsers,
                error_dialog,
                adw_application: adw_application.clone(),
                window,
                fetch,
                pages,
                assets,
            }
        })
    }

    pub fn init(self: &Rc<Self>) {
        if let Err(error) = (|| -> Result<()> {
            // Order matters!
            self.window.init(self);
            self.error_dialog.init(self);
            self.assets.init()?;
            self.browsers_configs.init(self);
            self.pages.init(self);

            self.navigate(&Page::Home);

            Ok(())
        })() {
            self.show_error(&error);
        }
    }

    pub fn navigate(self: &Rc<Self>, page: &Page) {
        self.window.view.navigate(self, page);
    }

    pub fn get_applications_dir(&self) -> Result<PathBuf> {
        if cfg!(debug_assertions) {
            let path = Path::new("./dev-assets/desktop-files").to_path_buf();
            debug!("Using dev applications path: {}", path.display());
            return Ok(path);
        }

        let Some(data_home_path) = self.dirs.get_data_home() else {
            bail!("Could not get data home path")
        };

        let path = data_home_path.join("applications");
        debug!("Using applications path: {}", path.display());

        if !path.is_dir() {
            bail!("Could not get applications path");
        }

        Ok(path)
    }

    pub fn get_icons_dir(&self) -> Result<PathBuf> {
        if cfg!(debug_assertions) {
            let path = Path::new("./dev-assets/icons").to_path_buf();
            debug!("Using dev icons path: {}", path.display());
            return Ok(path);
        }

        let Some(data_home_path) = self.dirs.get_data_home() else {
            bail!("Could not get data home path")
        };

        let path = data_home_path.join("icons");
        debug!("Using icons path: {}", path.display());

        Ok(path)
    }

    pub fn show_error(self: &Rc<Self>, error: &Error) {
        error!("{error}");
        self.error_dialog.show(self, error);
    }

    pub fn close(self: &Rc<Self>) {
        self.window.close();
    }

    pub fn restart(mut self: Rc<Self>) {
        self.close();
        let new_self = Self::new(&self.adw_application);
        self = new_self;
        self.init();
    }
}
