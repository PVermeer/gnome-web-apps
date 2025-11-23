mod error_dialog;
mod pages;
mod window;

use crate::services::{app_dirs::AppDirs, assets::Assets, browsers::BrowserConfigs, fetch::Fetch};
use anyhow::{Error, Result};
use error_dialog::ErrorDialog;
use log::error;
use pages::{Page, Pages};
use std::rc::Rc;
use window::AppWindow;

pub struct App {
    pub dirs: Rc<AppDirs>,
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
            let app_dirs = AppDirs::new();
            let window = AppWindow::new(adw_application);
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
            self.dirs.init()?;
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
