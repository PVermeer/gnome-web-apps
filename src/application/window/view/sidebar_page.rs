use std::rc::Rc;

use super::NavPage;
use crate::{
    application::{App, pages::Page},
    config,
};
use libadwaita::{
    ActionRow, HeaderBar, NavigationPage, ToolbarView,
    gtk::{ListBox, SelectionMode},
    prelude::ActionRowExt,
};

pub struct SidebarPage {
    pub nav_page: NavigationPage,
    pub header: HeaderBar,
    title: String,
    list: ListBox,
}
impl NavPage for SidebarPage {
    fn get_title(&self) -> &str {
        &self.title
    }

    fn get_navpage(&self) -> &NavigationPage {
        &self.nav_page
    }
}
impl SidebarPage {
    pub fn new() -> Self {
        let title = config::APP_NAME.to_string();
        let list = ListBox::builder()
            .selection_mode(SelectionMode::Single)
            .css_classes(["navigation-sidebar"])
            .build();
        let header = HeaderBar::new();
        let toolbar = ToolbarView::new();
        toolbar.add_top_bar(&header);
        toolbar.set_content(Some(&list));

        let nav_page = NavigationPage::builder()
            .title(&title)
            .tag("sidebar")
            .child(&toolbar)
            .build();

        return Self {
            nav_page,
            header,
            list,
            title,
        };
    }

    pub fn add_nav_row(&self, application: Rc<App>, page: Page) -> ActionRow {
        let nav_page = application.pages.get(page.clone());
        let row = ActionRow::builder()
            .activatable(true)
            .title(nav_page.get_title())
            .build();

        row.connect_activated(move |_| application.navigate(page.clone()));

        self.list.append(&row);
        row
    }
}
