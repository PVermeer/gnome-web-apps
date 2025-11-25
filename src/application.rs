mod error_dialog;
mod pages;
mod window;

use crate::services::{
    app_dirs::AppDirs, assets::Assets, browsers::BrowserConfigs, fetch::Fetch, utils,
};
use anyhow::{Error, Result};
use error_dialog::ErrorDialog;
use gtk::{IconTheme, gdk};
use pages::{Page, Pages};
use std::{path::Path, rc::Rc};
use tracing::{debug, error};
use window::AppWindow;

pub struct App {
    pub dirs: Rc<AppDirs>,
    pub browser_configs: Rc<BrowserConfigs>,
    pub error_dialog: ErrorDialog,
    adw_application: libadwaita::Application,
    icon_theme: IconTheme,
    window: AppWindow,
    fetch: Fetch,
    pages: Pages,
    assets: Assets,
}
impl App {
    pub fn new(adw_application: &libadwaita::Application) -> Rc<Self> {
        Rc::new({
            let icon_theme = gtk::IconTheme::for_display(
                &gdk::Display::default().expect("Could not connect to display"),
            );
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
                icon_theme,
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
            self.add_system_icon_paths();
            self.browser_configs.init(self);
            self.pages.init(self);

            self.navigate(&Page::Home);

            Ok(())
        })() {
            self.show_error(&error);
        }
    }

    pub fn add_icon_search_path(self: &Rc<Self>, path: &Path) {
        if !path.is_dir() {
            debug!("Not a valid icon path: {}", path.display());
            return;
        }

        debug!("Adding icon path to icon theme: {}", path.display());
        self.icon_theme.add_search_path(path);
    }

    pub fn has_icon(self: &Rc<Self>, icon: &str) -> bool {
        self.icon_theme.has_icon(icon)
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

    fn add_system_icon_paths(self: &Rc<Self>) {
        if utils::env::is_flatpak_container() {
            for path in self.dirs.system_icons() {
                self.add_icon_search_path(&path);
            }
        }
    }
}
