mod web_app_view;

use super::NavPage;
use super::PrefNavPage;
use crate::application::pages::web_apps::web_app_view::WebAppView;
use libadwaita::{
    ActionRow, ButtonContent, NavigationPage, NavigationView, PreferencesGroup, PreferencesPage,
    gtk::{Button, Image, prelude::ButtonExt},
    prelude::{ActionRowExt, PreferencesGroupExt, PreferencesPageExt},
};
use log::debug;
use std::rc::Rc;

pub struct WebAppsPage {
    nav_page: NavigationPage,
    nav_row: ActionRow,
    nav_view: NavigationView,
    prefs_page: PreferencesPage,
}
impl NavPage for WebAppsPage {
    fn get_navpage(&self) -> &NavigationPage {
        &self.nav_page
    }

    fn get_nav_row(&self) -> Option<&ActionRow> {
        Some(&self.nav_row)
    }
}
impl WebAppsPage {
    pub fn new() -> Rc<Self> {
        let title = "Web Apps";
        let icon = "preferences-desktop-apps-symbolic";

        let PrefNavPage {
            nav_page,
            nav_row,
            nav_view,
            prefs_page,
            ..
        } = Self::build_nav_page(title, icon).with_preference_navigation_view();

        Rc::new(Self {
            nav_page,
            nav_row,
            nav_view,
            prefs_page,
        })
    }

    pub fn init(self: &Rc<Self>) {
        let app_section = self.clone().build_apps_section();
        self.prefs_page.add(&app_section);
    }

    fn build_apps_section(self: Rc<Self>) -> PreferencesGroup {
        let button_content = ButtonContent::builder()
            .label("New app")
            .icon_name("list-add-symbolic")
            .build();
        let new_app_button = Button::builder()
            .css_classes(["flat"])
            .child(&button_content)
            .build();
        new_app_button.connect_clicked(|_| debug!("TODO"));

        let pref_group = PreferencesGroup::builder()
            .title("Apps")
            .header_suffix(&new_app_button)
            .build();
        let web_app_row = self.build_app_row("My web app");
        pref_group.add(&web_app_row);

        pref_group
    }

    fn build_app_row(self: Rc<Self>, title: &str) -> ActionRow {
        let app_row = ActionRow::builder().title(title).activatable(true).build();
        let prefix = Image::from_icon_name("web-browser-symbolic");
        let suffix = Image::from_icon_name("go-next-symbolic");
        app_row.add_prefix(&prefix);
        app_row.add_suffix(&suffix);

        app_row.connect_activated(move |_| {
            let app_page = WebAppView::new("Some web app");
            self.nav_view.push(app_page.get_navpage());
        });

        app_row
    }
}
