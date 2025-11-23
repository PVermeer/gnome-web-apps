mod error_dialog;
mod pages;
mod window;

use crate::{
    config,
    services::{assets::Assets, browsers::BrowserConfigs, fetch::Fetch, utils},
};
use anyhow::{Context, Error, Result, bail};
use error_dialog::ErrorDialog;
use log::{debug, error};
use pages::{Page, Pages};
use std::{
    os,
    path::{Path, PathBuf},
    rc::Rc,
};
use window::AppWindow;
use xdg::BaseDirectories;

pub struct App {
    pub dirs: Rc<BaseDirectories>,
    pub browser_configs: Rc<BrowserConfigs>,
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
            let error_dialog = ErrorDialog::new();
            let assets = Assets::new(&app_dirs);

            Self {
                dirs: app_dirs,
                browser_configs: browsers,
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
            self.browser_configs.init(self);
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
        let data_home_path = self
            .dirs
            .get_data_home()
            .context("Could not get data home path")?;
        let mut system_applications_path = utils::files::get_user_applications_dir()?;
        let mut app_applications_path = data_home_path.join("applications");

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
        error!("{error:?}");
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
