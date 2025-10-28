mod pages;
mod window;

use pages::{Page, Pages};
use std::rc::Rc;
use window::AppWindow;

pub struct App {
    window: AppWindow,
    pages: Pages,
}
impl App {
    pub fn new(adw_application: &libadwaita::Application) -> Rc<Self> {
        Rc::new({
            let pages = Pages::new();
            let window = AppWindow::new(adw_application);

            Self { window, pages }
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
