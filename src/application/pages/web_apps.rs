use super::NavPage;
use libadwaita::{
    ActionRow, ButtonContent, NavigationPage, PreferencesGroup,
    gtk::{
        Button, Image,
        prelude::{BoxExt, ButtonExt},
    },
    prelude::{ActionRowExt, PreferencesGroupExt},
};
use log::debug;

pub struct WebAppsPage {
    nav_page: NavigationPage,
    title: String,
    icon: String,
}
impl NavPage for WebAppsPage {
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
impl WebAppsPage {
    pub fn new() -> Self {
        let title = String::from("Web Apps");
        let icon = "preferences-desktop-apps-symbolic".to_string();
        let (nav_page, _header, content_box) = Self::build_nav_page(&title);
        let app_section = Self::build_apps_section();

        content_box.append(&app_section);

        Self {
            nav_page,
            title,
            icon,
        }
    }

    fn build_apps_section() -> PreferencesGroup {
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
        let web_app_row = Self::build_app_row("My web app");
        pref_group.add(&web_app_row);

        pref_group
    }

    fn build_app_row(title: &str) -> ActionRow {
        let row = ActionRow::builder().title(title).activatable(true).build();
        let prefix = Image::from_icon_name("web-browser-symbolic");
        let suffix = Image::from_icon_name("go-next-symbolic");
        row.add_prefix(&prefix);
        row.add_suffix(&suffix);

        row.connect_activated(|_| {
            debug!("TODO");
        });

        row
    }
}
