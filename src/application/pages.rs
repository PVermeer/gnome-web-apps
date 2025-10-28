mod home;
mod web_apps;

use home::HomePage;
use libadwaita::{
    ActionRow, Clamp, HeaderBar, NavigationPage, NavigationSplitView, ToolbarView,
    gtk::{self, Image, Orientation, ScrolledWindow, prelude::WidgetExt},
    prelude::ActionRowExt,
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

    fn get_nav_row(&self) -> Option<&ActionRow>;

    fn load_page(&self, view: &NavigationSplitView) {
        let nav_page = self.get_navpage();
        if nav_page.parent().is_some() {
            return;
        }
        view.set_content(Some(nav_page));
    }

    fn build_nav_page(title: &str, icon: &str) -> (NavigationPage, ActionRow, HeaderBar, gtk::Box)
    where
        Self: Sized,
    {
        const MARGIN: i32 = 20;
        const MAX_WIDTH: i32 = 600;

        let content_box = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .margin_top(MARGIN)
            .margin_bottom(MARGIN)
            .margin_start(MARGIN)
            .margin_end(MARGIN)
            .build();
        let clamp = Clamp::builder()
            .maximum_size(MAX_WIDTH)
            .child(&content_box)
            .build();
        let scrolled_window = ScrolledWindow::builder().child(&clamp).build();

        let header = HeaderBar::new();
        let toolbar = ToolbarView::new();
        toolbar.add_top_bar(&header);
        toolbar.set_content(Some(&scrolled_window));

        let nav_page = NavigationPage::builder()
            .title(title)
            .tag(title)
            .child(&toolbar)
            .build();

        let nav_row = ActionRow::builder().activatable(true).title(title).build();
        let icon_prefix = Image::from_icon_name(icon);
        nav_row.add_prefix(&icon_prefix);

        (nav_page, nav_row, header, content_box)
    }
}
