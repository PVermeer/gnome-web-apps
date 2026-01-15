mod app_menu;
mod sidebar_page;

use crate::application::{
    App,
    pages::{NavPage, Page},
};
use app_menu::AppMenu;
use gtk::{
    Button,
    prelude::{ButtonExt, WidgetExt},
};
use libadwaita::{Breakpoint, BreakpointCondition, NavigationSplitView, glib::Value};
use sidebar_page::SidebarPage;
use std::rc::Rc;

pub struct View {
    pub app_menu: AppMenu,
    pub sidebar: SidebarPage,
    pub nav_split: NavigationSplitView,
    pub breakpoint: Breakpoint,
    pub updated_button: Button,
}
impl View {
    pub fn new() -> Self {
        let sidebar = SidebarPage::new();
        let app_menu = AppMenu::new();
        let nav_split = NavigationSplitView::builder()
            .sidebar(&sidebar.nav_page)
            .show_content(true)
            .min_sidebar_width(250.0)
            .build();
        let breakpoint = Self::build_breakpoint();
        let updated_button = Self::build_updated_button();

        Self {
            app_menu,
            sidebar,
            nav_split,
            breakpoint,
            updated_button,
        }
    }

    pub fn init(&self, app: &Rc<App>) {
        self.app_menu.init(app);
        self.sidebar.header.pack_end(&self.app_menu.button);
        self.sidebar.header.pack_start(&self.updated_button);
        self.breakpoint
            .add_setter(&self.nav_split, "collapsed", Some(&Value::from(true)));
        self.connect_updated_button(app);
    }

    pub fn navigate(&self, app: &Rc<App>, page: &Page) {
        let nav_page = app.pages.get(page);
        nav_page.load_page(&self.nav_split);
        app.window.view.nav_split.set_show_content(true);
        app.window.view.sidebar.select_nav_row(app, page);
    }

    pub fn on_app_update(&self) {
        self.updated_button.set_visible(true);
    }

    fn build_breakpoint() -> Breakpoint {
        let breakpoint_condition = BreakpointCondition::new_length(
            libadwaita::BreakpointConditionLengthType::MaxWidth,
            600_f64,
            libadwaita::LengthUnit::Sp,
        );

        Breakpoint::new(breakpoint_condition)
    }

    fn build_updated_button() -> Button {
        Button::builder()
            .icon_name("software-update-available-symbolic")
            .css_classes(["accent", "flat"])
            .tooltip_text("Apps have been updated")
            .visible(false)
            .build()
    }

    fn connect_updated_button(&self, app: &Rc<App>) {
        let app_clone = app.clone();

        self.updated_button.connect_clicked(move |button| {
            app_clone.window.show_about();
            button.set_visible(false);
        });
    }
}
