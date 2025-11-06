mod icon_picker;

use super::WebAppsPage;
use crate::{
    application::{
        App,
        pages::{NavPage, PrefPage, web_apps::web_app_view::icon_picker::IconPicker},
    },
    config,
};
use freedesktop_desktop_entry::DesktopEntry;
use libadwaita::{
    ActionRow, ButtonContent, EntryRow, NavigationPage, PreferencesGroup, PreferencesPage, Toast,
    ToastOverlay, WrapBox,
    gtk::{
        self, Button, Image, InputPurpose, Label, Orientation,
        prelude::{BoxExt, ButtonExt, EditableExt, WidgetExt},
    },
    prelude::{EntryRowExt, PreferencesGroupExt, PreferencesPageExt, PreferencesRowExt},
};
use log::{debug, error};
use std::{borrow::Cow, cell::RefCell, process::Command, rc::Rc};
use validator::ValidateUrl;

pub struct WebAppView {
    nav_page: NavigationPage,
    desktop_file: Rc<RefCell<DesktopEntry>>,
    locales: Vec<String>,
    prefs_page: PreferencesPage,
    toast_overlay: ToastOverlay,
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
    pub fn new(desktop_file: &Rc<RefCell<DesktopEntry>>, locales: &[String]) -> Self {
        let desktop_file_borrow = desktop_file.borrow_mut();

        let title = desktop_file_borrow
            .name(locales)
            .unwrap_or(Cow::Borrowed("No name"));
        let icon = "preferences-desktop-apps-symbolic";

        let PrefPage {
            nav_page,
            prefs_page,
            toast_overlay,
            ..
        } = Self::build_nav_page(&title, icon).with_preference_page();

        drop(desktop_file_borrow);

        Self {
            nav_page,
            desktop_file: desktop_file.clone(),
            locales: locales.to_owned(),
            prefs_page,
            toast_overlay,
        }
    }

    pub fn init(&self, app: &Rc<App>) {
        let header = self.build_app_header();
        let general_pref_group = self.build_general_pref_group(app);

        self.prefs_page.add(&header);
        self.prefs_page.add(&general_pref_group);
    }

    fn build_app_header(&self) -> PreferencesGroup {
        let desktop_file_borrow = self.desktop_file.borrow_mut();

        let pref_group = PreferencesGroup::builder().build();
        let content_box = gtk::Box::new(Orientation::Vertical, 6);
        let app_name = desktop_file_borrow
            .name(&self.locales)
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
                debug!("Running app: '{} {}'", cmd, args.join(" "));

                #[allow(clippy::zombie_processes)]
                let result = Command::new(cmd.clone()).args(&args).spawn();

                if let Err(error) = result {
                    error!("Failed to run app '{} {}': {error:?}", cmd, args.join(" "));
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

    fn build_general_pref_group(&self, app: &Rc<App>) -> PreferencesGroup {
        let button_content = ButtonContent::builder()
            .label("Update icon")
            .icon_name("software-update-available-symbolic")
            .build();
        let edit_icon_button = Button::builder().child(&button_content).build();

        let app_clone = app.clone();
        let desktop_file_clone = self.desktop_file.clone();
        let toast_overlay_clone = self.toast_overlay.clone();
        edit_icon_button.connect_clicked(move |_| {
            let icon_picker = IconPicker::new(&desktop_file_clone);
            icon_picker.init(&app_clone, Some(&toast_overlay_clone));
            icon_picker.show_dialog(&app_clone);
        });

        let pref_group = PreferencesGroup::builder()
            .header_suffix(&edit_icon_button)
            .build();

        let url_row = self.build_url_row();
        pref_group.add(&url_row);

        pref_group
    }

    fn build_url_row(&self) -> EntryRow {
        let desktop_file_borrow = self.desktop_file.borrow();

        let url = desktop_file_borrow
            .desktop_entry(config::DesktopFile::URL_KEY)
            .unwrap_or_default();

        let entry_row = EntryRow::builder()
            .title("Website URL")
            .text(url)
            .show_apply_button(true)
            .input_purpose(InputPurpose::Url)
            .build();
        let validate_icon = Image::from_icon_name("dialog-warning-symbolic");
        validate_icon.set_visible(false);
        validate_icon.set_css_classes(&["error"]);
        entry_row.add_suffix(&validate_icon);

        drop(desktop_file_borrow);
        let desktop_file_clone = self.desktop_file.clone();
        let toast_overlay_clone = self.toast_overlay.clone();

        entry_row.connect_changed(move |entry_row| {
            let is_valid = entry_row.text().validate_url();

            debug!("{} is valid: {is_valid}", entry_row.title());

            validate_icon.set_visible(!entry_row.text().is_empty() && !is_valid);
            if is_valid {
                entry_row.set_show_apply_button(true);
                entry_row.set_tooltip_text(None);
            } else {
                entry_row.set_show_apply_button(false);
                entry_row
                    .set_tooltip_text(Some("Please enter a valid URL (e.g., https://example.com)"));
            }
        });

        entry_row.connect_apply(move |entry_row| {
            let mut desktop_file_borrow = desktop_file_clone.borrow_mut();
            let undo_text = desktop_file_borrow
                .desktop_entry(config::DesktopFile::URL_KEY)
                .unwrap_or_default()
                .to_string();

            desktop_file_borrow.add_desktop_entry(
                config::DesktopFile::URL_KEY.to_string(),
                entry_row.text().to_string(),
            );

            let entry_row_clone = entry_row.clone();
            let saved_toast = Toast::builder().title("Saved").build();
            saved_toast.set_button_label(Some("Undo"));
            saved_toast.connect_button_clicked(move |_| {
                entry_row_clone.set_text(&undo_text);
            });
            toast_overlay_clone.add_toast(saved_toast);

            debug!(
                "Set new URL on `desktop file`: {}",
                desktop_file_borrow
                    .desktop_entry(config::DesktopFile::URL_KEY)
                    .unwrap_or_default()
            );
        });

        entry_row
    }
}
