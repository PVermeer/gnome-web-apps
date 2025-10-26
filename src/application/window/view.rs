pub mod app_menu;
mod sidebar_page;

use crate::application::{
    App,
    pages::{NavPage, Page},
};
use app_menu::AppMenu;
use libadwaita::{Breakpoint, BreakpointCondition, NavigationSplitView, glib::Value};
use sidebar_page::SidebarPage;
use std::rc::Rc;

pub struct View {
    pub app_menu: AppMenu,
    pub sidebar: SidebarPage,
    pub split_view: NavigationSplitView,
    pub breakpoint: Breakpoint,
}
impl View {
    pub fn new() -> Self {
        let sidebar = SidebarPage::new();
        let app_menu = AppMenu::new();
        let split_view = NavigationSplitView::builder()
            .sidebar(&sidebar.nav_page)
            .show_content(true)
            .min_sidebar_width(250.0)
            .build();
        let breakpoint = Self::build_breakpoint();

        Self {
            app_menu,
            sidebar,
            split_view,
            breakpoint,
        }
    }

    pub fn init(&self, app: Rc<App>) {
        self.app_menu.init(app);
        self.sidebar.header.pack_end(&self.app_menu.button);
        self.breakpoint
            .add_setter(&self.split_view, "collapsed", Some(&Value::from(true)));
    }

    pub fn navigate(&self, app: Rc<App>, page: Page) {
        app.pages.get(page).load_page(&self.split_view);
        app.app_window.view.split_view.set_show_content(true);
    }

    fn build_breakpoint() -> Breakpoint {
        let breakpoint_condition = BreakpointCondition::new_length(
            libadwaita::BreakpointConditionLengthType::MaxWidth,
            600_f64,
            libadwaita::LengthUnit::Sp,
        );
        let breakpoint = Breakpoint::new(breakpoint_condition);

        breakpoint
    }
}
