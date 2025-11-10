mod browser_configs;
mod fetch;
mod pages;
mod window;

use crate::{application::browser_configs::BrowserConfigs, config};
use fetch::Fetch;
use pages::{Page, Pages};
use std::rc::Rc;
use window::AppWindow;
use xdg::BaseDirectories;

pub struct App {
    window: AppWindow,
    dirs: BaseDirectories,
    fetch: Fetch,
    pages: Pages,
    browsers_configs: BrowserConfigs,
}
impl App {
    pub fn new(adw_application: &libadwaita::Application) -> Rc<Self> {
        Rc::new({
            let window = AppWindow::new(adw_application);
            let app_dirs = BaseDirectories::with_prefix(config::APP_NAME_PATH);
            let fetch = Fetch::new();
            let pages = Pages::new();
            let browsers = BrowserConfigs::new();

            Self {
                window,
                dirs: app_dirs,
                fetch,
                pages,
                browsers_configs: browsers,
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
}
