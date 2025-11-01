use super::WebAppsPage;
use crate::{
    application::pages::{NavPage, PrefPage},
    config,
};
use freedesktop_desktop_entry::DesktopEntry;
use libadwaita::{
    ActionRow, ButtonContent, EntryRow, NavigationPage, PreferencesGroup, WrapBox,
    gtk::{
        self, Button, InputPurpose, Label, Orientation,
        prelude::{BoxExt, ButtonExt, EditableExt, WidgetExt},
    },
    prelude::{EntryRowExt, PreferencesGroupExt, PreferencesPageExt},
};
use log::{debug, error};
use std::{borrow::Cow, cell::RefCell, process::Command, rc::Rc};

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
    const LABEL: &str = "web-app-view";
    const LOG_TARGET: &str = Self::LABEL;

    pub fn new(desktop_file: &Rc<RefCell<DesktopEntry>>, locales: &[String]) -> Self {
        let desktop_file_borrow = desktop_file.borrow_mut();

        let title = desktop_file_borrow
            .name(locales)
            .unwrap_or(Cow::Borrowed("No name"));
        let icon = "preferences-desktop-apps-symbolic";

        let PrefPage {
            nav_page,
            prefs_page,
            ..
        } = Self::build_nav_page(&title, icon).with_preference_page();

        drop(desktop_file_borrow);

        let header = Self::build_app_header(desktop_file, locales);
        let general_pref_group = Self::build_general_pref_group(desktop_file);

        prefs_page.add(&header);
        prefs_page.add(&general_pref_group);

        Self { nav_page }
    }

    fn build_app_header(
        desktop_file: &Rc<RefCell<DesktopEntry>>,
        locales: &[String],
    ) -> PreferencesGroup {
        let desktop_file_borrow = desktop_file.borrow_mut();

        let pref_group = PreferencesGroup::builder().build();
        let content_box = gtk::Box::new(Orientation::Vertical, 6);
        let app_name = desktop_file_borrow
            .name(locales)
            .unwrap_or(Cow::Borrowed("No name..."));
        let app_label = Label::builder()
            .label(app_name)
            .css_classes(["title-1"])
            .build();

        let run_button = Button::builder()
            .label("Open")
            .css_classes(["suggested-action", "pill"])
            .build();
        let mut exec_args = desktop_file_borrow.parse_exec().unwrap_or_default();
        let command = if exec_args.is_empty() {
            run_button.set_sensitive(false);
            None
        } else {
            Some(exec_args.remove(0))
        };
        let args = exec_args;
        if let Some(cmd) = command {
            run_button.connect_clicked(move |_| {
                debug!(target: Self::LOG_TARGET, "Running app: '{} {}'", cmd, args.join(" "));

                #[allow(clippy::zombie_processes)]
                let result = Command::new(cmd.clone()).args(&args).spawn();

                if let Err(error) = result {
                    error!(target: Self::LOG_TARGET, "Failed to run app '{} {}': {error}", cmd, args.join(" "));
                }
            });
        }
        let button_wrap_box = WrapBox::builder()
            .align(0.5)
            .margin_top(12)
            .margin_bottom(12)
            .build();
        button_wrap_box.append(&run_button);

        let app_image = WebAppsPage::get_image_icon(&desktop_file_borrow);
        app_image.set_css_classes(&["icon-dropshadow"]);
        app_image.set_pixel_size(96);
        app_image.set_margin_start(25);
        app_image.set_margin_end(25);

        content_box.append(&app_image);
        content_box.append(&app_label);
        content_box.append(&button_wrap_box);

        pref_group.add(&content_box);

        pref_group
    }

    fn build_general_pref_group(desktop_file: &Rc<RefCell<DesktopEntry>>) -> PreferencesGroup {
        let button_content = ButtonContent::builder()
            .label("Update icon")
            .icon_name("software-update-available-symbolic")
            .build();
        let edit_icon_button = Button::builder().child(&button_content).build();
        edit_icon_button.connect_clicked(|_| debug!("TODO"));

        let pref_group = PreferencesGroup::builder()
            .header_suffix(&edit_icon_button)
            .build();

        let url_row = Self::build_url_row(desktop_file);
        pref_group.add(&url_row);

        pref_group
    }

    fn build_url_row(desktop_file: &Rc<RefCell<DesktopEntry>>) -> EntryRow {
        let desktop_file_borrow = desktop_file.borrow();

        let url = desktop_file_borrow
            .desktop_entry(config::DesktopFile::URL_KEY)
            .unwrap_or_default();

        let row = EntryRow::builder()
            .title("URL")
            .text(url)
            .input_purpose(InputPurpose::Url)
            .show_apply_button(true)
            .build();

        drop(desktop_file_borrow);
        let desktop_file_cloned = desktop_file.clone();

        row.connect_apply(move |entry_row| {
            let mut desktop_file_borrow = desktop_file_cloned.borrow_mut();

            desktop_file_borrow.add_desktop_entry(
                config::DesktopFile::URL_KEY.to_string(),
                entry_row.text().to_string(),
            );

            debug!(
                target: Self::LOG_TARGET,
                "Set new URL on `desktop file`: {}",
                desktop_file_borrow.desktop_entry(config::DesktopFile::URL_KEY).unwrap_or_default()
            );
        });

        row
    }
}
