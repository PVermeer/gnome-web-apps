mod home;
mod web_apps;

use home::HomePage;
use libadwaita::{
    HeaderBar, NavigationPage, NavigationSplitView, ToolbarView,
    gtk::{self, Orientation, prelude::WidgetExt},
};
use std::rc::Rc;
use web_apps::WebAppsPage;

use crate::application::App;

#[derive(Clone)]
#[repr(i32)]
pub enum Page {
    Home,
    WebApps,
}

pub struct Pages {
    home: Rc<HomePage>,
    web_apps: Rc<WebAppsPage>,
}
#[allow(clippy::unused_self)]
impl Pages {
    pub fn new() -> Self {
        Self {
            home: Rc::new(HomePage::new()),
            web_apps: Rc::new(WebAppsPage::new()),
        }
    }

    pub fn init(&self, app: &Rc<App>) {
        let sidebar = &app.window.view.sidebar;

        sidebar.add_nav_row(app.clone(), Page::Home);
        sidebar.add_nav_row(app.clone(), Page::WebApps);
    }

    pub fn get(&self, page: &Page) -> Rc<dyn NavPage> {
        match page {
            Page::Home => self.home.clone(),
            Page::WebApps => self.web_apps.clone(),
        }
    }
}

pub trait NavPage {
    fn get_navpage(&self) -> &NavigationPage;

    fn get_title(&self) -> &str;

    /**
    Icon name from Adwaita icon list.
    */
    fn get_icon(&self) -> &str;

    fn load_page(&self, view: &NavigationSplitView) {
        let nav_page = self.get_navpage();
        if nav_page.parent().is_some() {
            return;
        }
        view.set_content(Some(nav_page));
    }

    fn build_nav_page(title: &str) -> (NavigationPage, HeaderBar, gtk::Box)
    where
        Self: Sized,
    {
        const MARGIN: i32 = 20;

        let content_box = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .margin_top(MARGIN)
            .margin_bottom(MARGIN)
            .margin_start(MARGIN)
            .margin_end(MARGIN)
            .build();

        let header = HeaderBar::new();
        let toolbar = ToolbarView::new();
        toolbar.add_top_bar(&header);
        toolbar.set_content(Some(&content_box));

        let nav_page = NavigationPage::builder()
            .title(title)
            .tag(title)
            .child(&toolbar)
            .build();

        (nav_page, header, content_box)
    }
}
