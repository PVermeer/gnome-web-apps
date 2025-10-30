mod web_app_view;

use super::NavPage;
use super::PrefNavPage;
use crate::application::App;
use crate::application::pages::web_apps::web_app_view::WebAppView;
use crate::config;
use anyhow::Context;
use anyhow::Result;
use freedesktop_desktop_entry::DesktopEntry;
use freedesktop_desktop_entry::get_languages_from_env;
use libadwaita::{
    ActionRow, ButtonContent, NavigationPage, NavigationView, PreferencesGroup, PreferencesPage,
    gtk::{Button, Image, prelude::ButtonExt},
    prelude::{ActionRowExt, PreferencesGroupExt, PreferencesPageExt},
};
use log::debug;
use std::borrow::Cow;
use std::path::Path;
use std::rc::Rc;

pub struct WebAppsPage {
    nav_page: NavigationPage,
    nav_row: ActionRow,
    nav_view: NavigationView,
    prefs_page: PreferencesPage,
    locales: Vec<String>,
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
    const LABEL: &str = "web-apps-page";
    const LOG_TARGET: &str = Self::LABEL;

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

        let locales = get_languages_from_env();

        Rc::new(Self {
            nav_page,
            nav_row,
            nav_view,
            prefs_page,
            locales,
        })
    }

    pub fn init(self: &Rc<Self>, app: &Rc<App>) {
        let app_section = self.clone().build_apps_section(app);
        self.prefs_page.add(&app_section);
    }

    fn build_apps_section(self: Rc<Self>, app: &Rc<App>) -> PreferencesGroup {
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

        // TODO remove unwraps
        for desktop_file in self.get_owned_desktop_files(app).unwrap() {
            let web_app_row = self.clone().build_app_row(desktop_file);
            pref_group.add(&web_app_row);
        }

        pref_group
    }

    fn build_app_row(self: Rc<Self>, desktop_file: Rc<DesktopEntry>) -> ActionRow {
        let app_name = desktop_file
            .name(&self.locales)
            .unwrap_or(Cow::Borrowed("No name"));
        let app_row = ActionRow::builder()
            .title(app_name)
            .activatable(true)
            .build();

        let app_icon = Self::get_image_icon(&desktop_file);
        let suffix = Image::from_icon_name("go-next-symbolic");

        app_row.add_prefix(&app_icon);
        app_row.add_suffix(&suffix);

        app_row.connect_activated(move |_| {
            let app_page = WebAppView::new(&desktop_file.clone(), &self.locales);
            self.nav_view.push(app_page.get_navpage());
        });

        app_row
    }

    fn get_owned_desktop_files(self: &Rc<Self>, app: &Rc<App>) -> Result<Vec<Rc<DesktopEntry>>> {
        debug!(target: Self::LOG_TARGET, "Reading user desktop files");

        let applications_path = app
            .dirs
            .data_home
            .as_ref()
            .context("There should be a user data dir!")?
            .join("applications");

        debug!(target: Self::LOG_TARGET, "Using path: {}", applications_path.display());
        applications_path
            .is_dir()
            .then_some(())
            .context("Path is not a directory!")?;

        let owned_web_app_key = "X-".to_string() + config::APP_NAME_PATH;
        let mut owned_desktop_files = Vec::new();

        for file in applications_path.read_dir().unwrap().flatten() {
            let Ok(desktop_file) = DesktopEntry::from_path(file.path(), Some(&self.locales)) else {
                continue;
            };

            if desktop_file
                .desktop_entry(&owned_web_app_key)
                .is_none_or(|value| value != "true")
            {
                continue;
            }

            debug!(target: Self::LOG_TARGET, "Found desktop file: {}", desktop_file.path.display());

            owned_desktop_files.push(Rc::new(desktop_file));
        }

        Ok(owned_desktop_files)
    }

    fn get_image_icon(desktop_file: &DesktopEntry) -> Image {
        let icon_name = desktop_file.icon().unwrap_or("image-missing-symbolic");
        let icon_path = Path::new(icon_name);
        if icon_path.is_file() {
            Image::from_file(icon_path)
        } else {
            Image::from_icon_name(icon_name)
        }
    }
}
