use crate::application::pages::{NavPage, PrefPage};
use libadwaita::{
    ActionRow, NavigationPage, PreferencesGroup,
    gtk::Image,
    prelude::{ActionRowExt, PreferencesGroupExt, PreferencesPageExt},
};
use log::debug;

pub struct WebAppView {
    nav_page: NavigationPage,
}
impl NavPage for WebAppView {
    fn get_navpage(&self) -> &NavigationPage {
        &self.nav_page
    }

    fn get_nav_row(&self) -> Option<&ActionRow> {
        None
    }
}
impl WebAppView {
    pub fn new(name: &str) -> Self {
        let title = name;
        let icon = "preferences-desktop-apps-symbolic";

        let PrefPage {
            nav_page,
            prefs_page,
            ..
        } = Self::build_nav_page(title, icon).with_preference_page();

        let web_app_pref_group = Self::build_app_pref_group();
        prefs_page.add(&web_app_pref_group);

        Self { nav_page }
    }

    fn build_app_pref_group() -> PreferencesGroup {
        let pref_group = PreferencesGroup::builder()
            .title("My awesome web app")
            .build();

        let row = ActionRow::builder().title("Some row").build();
        let prefix = Image::from_icon_name("web-browser-symbolic");
        row.add_prefix(&prefix);

        row.connect_activated(|_| {
            debug!("TODO");
        });

        pref_group.add(&row);

        pref_group
    }
}
