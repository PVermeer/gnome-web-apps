mod pages;
mod window;

use pages::{Page, Pages};
use std::rc::{Rc, Weak};
use window::AppWindow;

pub struct App {
    self_weak: Weak<App>,
    app_window: AppWindow,
    pages: Pages,
}
impl App {
    pub fn new(adw_application: libadwaita::Application) -> Rc<Self> {
        Rc::new_cyclic(|self_weak| {
            let pages = Pages::new();
            let window = AppWindow::new(&adw_application);

            Self {
                self_weak: self_weak.to_owned(),
                app_window: window,
                pages,
            }
        })
    }

    pub fn init(&self) {
        let app = self.get_app();
        app.app_window.init(app.clone());
        app.pages.init(app.clone());

        self.navigate(Page::Main);
    }

    pub fn get_app(&self) -> Rc<App> {
        self.self_weak.upgrade().unwrap()
    }

    fn navigate(&self, page: Page) {
        let app = self.get_app();
        self.app_window.view.navigate(app, page);
    }
}
