mod pages;
mod window;

use crate::config;
use pages::{Page, Pages};
use std::rc::Rc;
use window::AppWindow;
use xdg::BaseDirectories;

pub struct App {
    window: AppWindow,
    dirs: BaseDirectories,
    pages: Pages,
}
impl App {
    pub fn new(adw_application: &libadwaita::Application) -> Rc<Self> {
        Rc::new({
            let pages = Pages::new();
            let window = AppWindow::new(adw_application);
            let app_dirs = BaseDirectories::with_prefix(config::APP_NAME_PATH);

            Self {
                window,
                dirs: app_dirs,
                pages,
            }
        })
    }

    pub fn init(self: &Rc<Self>) {
        self.window.init(self);
        self.pages.init(self);

        self.navigate(&Page::Home);
    }

    pub fn navigate(self: &Rc<Self>, page: &Page) {
        self.window.view.navigate(self, page);
    }
}
