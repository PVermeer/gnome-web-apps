mod browsers;
mod home;
mod web_apps;

use crate::application::{App, pages::browsers::Browsers};
use home::HomePage;
use libadwaita::{
    ActionRow, Clamp, HeaderBar, NavigationPage, NavigationSplitView, NavigationView,
    PreferencesPage, ToastOverlay, ToolbarView,
    gtk::{self, Image, Orientation, ScrolledWindow, prelude::WidgetExt},
    prelude::ActionRowExt,
};
use std::rc::Rc;
use web_apps::WebAppsPage;

#[derive(Clone)]
#[repr(i32)]
pub enum Page {
    Home,
    WebApps,
    Browsers,
}

pub struct Pages {
    home: Rc<HomePage>,
    web_apps: Rc<WebAppsPage>,
    browsers: Rc<Browsers>,
}
#[allow(clippy::unused_self)]
impl Pages {
    pub fn new() -> Self {
        Self {
            home: HomePage::new(),
            web_apps: WebAppsPage::new(),
            browsers: Browsers::new(),
        }
    }

    pub fn init(&self, app: &Rc<App>) {
        self.home.init(app);
        self.web_apps.init(app);
        self.browsers.init(app);

        let sidebar = &app.window.view.sidebar;
        sidebar.add_nav_row(app.clone(), Page::Home);
        sidebar.add_nav_row(app.clone(), Page::WebApps);
        sidebar.add_nav_row(app.clone(), Page::Browsers);
    }

    pub fn get(&self, page: &Page) -> Rc<dyn NavPage> {
        match page {
            Page::Home => self.home.clone(),
            Page::WebApps => self.web_apps.clone(),
            Page::Browsers => self.browsers.clone(),
        }
    }
}

struct ContentPage {
    nav_page: NavigationPage,
    nav_row: ActionRow,
    content_box: gtk::Box,
}
struct PrefPage {
    nav_page: NavigationPage,
    nav_row: ActionRow,
    prefs_page: PreferencesPage,
    toast_overlay: ToastOverlay,
    header: HeaderBar,
}
struct PrefNavPage {
    nav_page: NavigationPage,
    nav_row: ActionRow,
    nav_view: NavigationView,
    prefs_page: PreferencesPage,
}
pub struct PageBuilder {
    nav_page: NavigationPage,
    nav_row: ActionRow,
    toolbar: ToolbarView,
    header: HeaderBar,
}
impl PageBuilder {
    fn with_content_box(self) -> ContentPage {
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
        self.toolbar.set_content(Some(&scrolled_window));

        ContentPage {
            nav_page: self.nav_page,
            nav_row: self.nav_row,
            content_box,
        }
    }

    fn with_preference_page(self) -> PrefPage {
        let prefs_page = PreferencesPage::new();
        let toast_overlay = ToastOverlay::new();
        toast_overlay.set_child(Some(&prefs_page));
        self.toolbar.set_content(Some(&toast_overlay));

        PrefPage {
            nav_page: self.nav_page,
            nav_row: self.nav_row,
            prefs_page,
            toast_overlay,
            header: self.header,
        }
    }

    /// This has a `NavigationView` for animations deeper in settings
    fn with_preference_navigation_view(self) -> PrefNavPage {
        let nav_view = NavigationView::new();
        let prefs_page = PreferencesPage::new();
        let nav_view_page = NavigationPage::builder().child(&nav_view).build();
        self.toolbar.set_content(Some(&prefs_page));
        nav_view.add(&self.nav_page);

        PrefNavPage {
            nav_page: nav_view_page,
            nav_row: self.nav_row,
            nav_view,
            prefs_page,
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

    fn build_nav_page(title: &str, icon: &str) -> PageBuilder
    where
        Self: Sized,
    {
        let header = HeaderBar::new();
        let toolbar = ToolbarView::new();
        toolbar.add_top_bar(&header);

        let nav_page = NavigationPage::builder()
            .title(title)
            .tag(title)
            .child(&toolbar)
            .build();

        let nav_row = ActionRow::builder().activatable(true).title(title).build();
        let icon_prefix = Image::from_icon_name(icon);
        nav_row.add_prefix(&icon_prefix);

        PageBuilder {
            nav_page,
            nav_row,
            toolbar,
            header,
        }
    }
}
