use std::rc::Rc;

use super::NavPage;
use crate::{
    application::{App, pages::Page},
    config,
};
use libadwaita::{
    ActionRow, HeaderBar, NavigationPage, ToolbarView,
    gtk::{Image, ListBox, SelectionMode},
    prelude::ActionRowExt,
};

pub struct SidebarPage {
    pub nav_page: NavigationPage,
    pub header: HeaderBar,
    title: String,
    icon: String,
    list: ListBox,
}
impl NavPage for SidebarPage {
    fn get_title(&self) -> &str {
        &self.title
    }

    fn get_icon(&self) -> &str {
        &self.icon
    }

    fn get_navpage(&self) -> &NavigationPage {
        &self.nav_page
    }
}
impl SidebarPage {
    pub fn new() -> Self {
        let title = config::APP_NAME.to_string();
        let icon = String::new();
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

        Self {
            nav_page,
            header,
            title,
            icon,
            list,
        }
    }

    pub fn add_nav_row(&self, app: Rc<App>, page: Page) -> ActionRow {
        let nav_page = app.pages.get(&page);
        let row = ActionRow::builder()
            .activatable(true)
            .title(nav_page.get_title())
            .build();
        let icon = Image::from_icon_name(nav_page.get_icon());
        row.add_prefix(&icon);

        row.connect_activated(move |_| app.navigate(&page.clone()));

        self.list.append(&row);
        row
    }
}
