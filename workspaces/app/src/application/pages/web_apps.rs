mod web_app_view;

use super::NavPage;
use crate::application::{App, pages::PrefNavPage};
use gtk::{
    Button, Image,
    prelude::{ButtonExt, WidgetExt},
};
use libadwaita::{
    ActionRow, ButtonContent, NavigationPage, NavigationView, PreferencesGroup, PreferencesPage,
    StatusPage,
    prelude::{ActionRowExt, PreferencesGroupExt, PreferencesPageExt},
};
use common::{desktop_file::DesktopFile, utils};
use std::{cell::RefCell, rc::Rc};
use tracing::debug;
use web_app_view::WebAppView;

pub struct WebAppsPage {
    nav_page: NavigationPage,
    nav_row: ActionRow,
    nav_view: Rc<NavigationView>,
    prefs_page: PreferencesPage,
    app_section: RefCell<PreferencesGroup>,
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
        let app_section = RefCell::new(PreferencesGroup::new());

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
            nav_view: Rc::new(nav_view),
            prefs_page,
            app_section,
        })
    }

    pub fn init(self: &Rc<Self>, app: &Rc<App>) {
        let app_section = self.clone().build_apps_section(app);
        self.prefs_page.add(&app_section);
        *self.app_section.borrow_mut() = app_section;

        let self_clone = self.clone();
        let app_clone = app.clone();

        self.nav_view
            .connect_popped(move |_, _| self_clone.reset_app_section(&app_clone));
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

        let self_clone = self.clone();
        let app_clone = app.clone();

        new_app_button.connect_clicked(move |_| {
            let desktop_file = Rc::new(RefCell::new(DesktopFile::new(
                &app_clone.browser_configs,
                &app_clone.dirs,
            )));
            let app_page = WebAppView::new(&app_clone, &self_clone.nav_view, &desktop_file, true);
            app_page.init();

            let nav_page = app_page.get_navpage();
            let app_page_clone = app_page.clone();
            nav_page.connect_unrealize(move |_| {
                if app_page_clone.get_is_new() {
                    let _ = desktop_file.borrow().delete();
                }
            });

            self_clone.nav_view.push(nav_page);
        });

        let pref_group = PreferencesGroup::builder()
            .header_suffix(&new_app_button)
            .build();

        let web_app_desktop_files = Self::get_owned_desktop_files(app);
        if web_app_desktop_files.is_empty() {
            let status_page = StatusPage::builder()
                .title("No Web Apps found")
                .description("Try adding one!")
                .icon_name("system-search-symbolic")
                .build();

            pref_group.add(&status_page);
        } else {
            for desktop_file in web_app_desktop_files {
                let web_app_row = self.clone().build_app_row(app, desktop_file);
                pref_group.add(&web_app_row);
            }
        }

        pref_group
    }

    fn build_app_row(
        self: Rc<Self>,
        app: &Rc<App>,
        desktop_file: Rc<RefCell<DesktopFile>>,
    ) -> ActionRow {
        let desktop_file_borrow = desktop_file.borrow();

        let app_name = desktop_file_borrow
            .get_name()
            .unwrap_or("No name".to_string());
        let app_row = ActionRow::builder()
            .title(app_name)
            .activatable(true)
            .build();

        let app_icon = desktop_file_borrow.get_icon();
        let suffix = Image::from_icon_name("go-next-symbolic");

        app_row.add_prefix(&app_icon);
        app_row.add_suffix(&suffix);

        drop(desktop_file_borrow);
        let app_clone = app.clone();
        let nav_view_clone = self.nav_view.clone();

        app_row.connect_activated(move |_| {
            let app_page =
                WebAppView::new(&app_clone, &nav_view_clone, &desktop_file.clone(), false);
            app_page.init();
            self.nav_view.push(app_page.get_navpage());
        });

        app_row
    }

    fn get_owned_desktop_files(app: &Rc<App>) -> Vec<Rc<RefCell<DesktopFile>>> {
        debug!("Reading user desktop files");

        let mut owned_desktop_files = Vec::new();
        let applications_path = app.dirs.applications();

        for file in utils::files::get_entries_in_dir(&applications_path).unwrap_or_default() {
            let Ok(desktop_file) =
                DesktopFile::from_path(&file.path(), &app.browser_configs, &app.dirs)
            else {
                continue;
            };

            if !desktop_file.get_is_owned_app() {
                continue;
            }

            debug!("Found desktop file: {}", desktop_file.get_path().display());

            owned_desktop_files.push(Rc::new(RefCell::new(desktop_file)));
        }

        *app.has_created_apps.borrow_mut() = !owned_desktop_files.is_empty();

        owned_desktop_files
    }

    fn reset_app_section(self: &Rc<Self>, app: &Rc<App>) {
        self.prefs_page.remove(&*self.app_section.borrow());
        *self.app_section.borrow_mut() = self.clone().build_apps_section(app);
        self.prefs_page.add(&*self.app_section.borrow());
    }
}
