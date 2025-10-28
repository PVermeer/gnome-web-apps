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
    nav_row: ActionRow,
    list: ListBox,
}
impl NavPage for SidebarPage {
    fn get_navpage(&self) -> &NavigationPage {
        &self.nav_page
    }

    fn get_nav_row(&self) -> Option<&ActionRow> {
        Some(&self.nav_row)
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
        let nav_row = ActionRow::new();

        Self {
            nav_page,
            header,
            nav_row,
            list,
        }
    }

    pub fn add_nav_row(&self, app: Rc<App>, page: Page) {
        let nav_page = app.pages.get(&page);
        let nav_row = nav_page.get_nav_row();
        if let Some(row) = nav_row {
            row.connect_activated(move |_| app.navigate(&page.clone()));
            self.list.append(row);
        }
    }

    pub fn select_nav_row(&self, app: &Rc<App>, page: &Page) {
        let nav_page = app.pages.get(page);
        let nav_row = nav_page.get_nav_row();
        if let Some(row) = nav_row {
            self.list.select_row(Some(row));
        }
    }
}
