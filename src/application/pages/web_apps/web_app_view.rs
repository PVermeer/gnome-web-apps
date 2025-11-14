mod icon_picker;

use super::WebAppsPage;
use crate::{
    application::{
        App,
        browser_configs::Browser,
        pages::{NavPage, PrefPage, web_apps::web_app_view::icon_picker::IconPicker},
    },
    config,
};
use freedesktop_desktop_entry::DesktopEntry;
use gtk::{
    ListItem, SignalListItemFactory, gio,
    glib::{BoxedAnyObject, object::Cast},
    prelude::ListItemExt,
};
use libadwaita::{
    ActionRow, ButtonContent, ComboRow, EntryRow, HeaderBar, NavigationPage, PreferencesGroup,
    PreferencesPage, SwitchRow, Toast, ToastOverlay, WrapBox,
    gtk::{
        self, Button, Image, InputPurpose, Label, Orientation,
        prelude::{BoxExt, ButtonExt, EditableExt, WidgetExt},
    },
    prelude::{
        AlertDialogExt, ComboRowExt, EntryRowExt, PreferencesGroupExt, PreferencesPageExt,
        PreferencesRowExt,
    },
};
use log::{debug, error};
use std::{
    borrow::Cow,
    cell::{Cell, RefCell},
    process::Command,
    rc::Rc,
};
use validator::ValidateUrl;

pub struct WebAppView {
    nav_page: NavigationPage,
    app: Rc<App>,
    header: HeaderBar,
    desktop_file: Rc<RefCell<DesktopEntry>>,
    desktop_file_original: DesktopEntry,
    locales: Vec<String>,
    prefs_page: PreferencesPage,
    pref_groups: RefCell<Vec<PreferencesGroup>>,
    toast_overlay: ToastOverlay,
    reset_button: Button,
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
    const TOAST_MESSAGE_TIMEOUT: u32 = 2;
    const TOAST_UNDO_TIMEOUT: u32 = 4;
    const TOAST_SAVED: &str = "Saved";
    const TOAST_RESET: &str = "Reset";
    const TOAST_UNDO_BUTTON: &str = "Undo";

    pub fn new(
        app: &Rc<App>,
        desktop_file: &Rc<RefCell<DesktopEntry>>,
        locales: &[String],
    ) -> Rc<Self> {
        let desktop_file_borrow = desktop_file.borrow();
        let desktop_file_original = desktop_file_borrow.clone(); // Deep clone
        let title = desktop_file_borrow
            .name(locales)
            .unwrap_or(Cow::Borrowed("No name"));
        let icon = "preferences-desktop-apps-symbolic";
        let PrefPage {
            nav_page,
            prefs_page,
            toast_overlay,
            header,
            ..
        } = Self::build_nav_page(&title, icon).with_preference_page();
        let reset_button = Self::build_header_reset_button();

        Rc::new(Self {
            nav_page,
            app: app.clone(),
            header,
            desktop_file: desktop_file.clone(),
            desktop_file_original,
            locales: locales.to_owned(),
            prefs_page,
            pref_groups: RefCell::new(Vec::new()),
            toast_overlay,
            reset_button,
        })
    }

    /// Init may be run sequentially to reset the view.
    pub fn init(self: &Rc<Self>) {
        let self_clone = self.clone();
        let mut pref_groups = self.pref_groups.borrow_mut();

        if pref_groups.is_empty() {
            // First init
            self.header.pack_end(&self.reset_button);
            self.reset_button
                .connect_clicked(move |_| self_clone.reset_desktop_file());
        } else {
            // Sequential init
            for pref_group in pref_groups.iter() {
                self.prefs_page.remove(pref_group);
            }
            pref_groups.clear();
        }

        let web_app_header = self.build_app_header();
        let general_pref_group = self.build_general_pref_group();

        pref_groups.push(web_app_header);
        pref_groups.push(general_pref_group);

        for pref_group in pref_groups.iter() {
            self.prefs_page.add(pref_group);
        }
    }

    fn reset_desktop_file(self: &Rc<Self>) {
        debug!("Resetting desktop file");

        *self.desktop_file.borrow_mut() = self.desktop_file_original.clone();
        self.reset_view();

        let toast = Self::build_reset_toast();
        self.toast_overlay.add_toast(toast);
    }

    fn build_header_reset_button() -> Button {
        let reset_button = Button::with_label("Reset");
        reset_button.set_sensitive(false);

        reset_button
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

        if let Some(exec) = desktop_file_borrow.exec() {
            let executable = exec.to_string();

            run_button.connect_clicked(move |_| {
                debug!("Running web app: '{executable}'");

                #[allow(clippy::zombie_processes)]
                let result = Command::new("sh").arg("-c").arg(executable.clone()).spawn();

                if let Err(error) = result {
                    error!("Failed to run app '{executable}': {error:?}");
                }
            });
        } else {
            run_button.set_sensitive(false);
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

        let browser_label = Label::new(None);
        let browser_id = desktop_file_borrow
            .desktop_entry(config::DesktopFile::BROWSER_ID)
            .unwrap_or_default();
        if let Some(browser) = self.app.browsers_configs.get_by_id(browser_id) {
            browser_label.set_markup(&format!("<b>{}</b>", &browser.get_name_with_installation()));
        }

        content_box.append(&app_image);
        content_box.append(&app_label);
        content_box.append(&browser_label);
        content_box.append(&button_wrap_box);

        pref_group.add(&content_box);

        pref_group
    }

    fn build_general_pref_group(self: &Rc<Self>) -> PreferencesGroup {
        let update_icon_button = self.build_update_icon_button();
        let pref_group = PreferencesGroup::builder()
            .header_suffix(&update_icon_button)
            .build();

        let name_row = self.build_name_row();
        let url_row = self.build_url_row();
        let browser_row = self.build_browser_row();
        let isolate_row = self.build_isolate_row();

        pref_group.add(&name_row);
        pref_group.add(&url_row);
        pref_group.add(&isolate_row);
        pref_group.add(&browser_row);

        pref_group
    }

    fn build_input_row(
        self: &Rc<Self>,
        row_title: &str,
        purpose: InputPurpose,
        desktop_file_key: &str,
    ) -> EntryRow {
        let desktop_file_borrow = self.desktop_file.borrow();
        let name = desktop_file_borrow
            .desktop_entry(desktop_file_key)
            .unwrap_or_default();

        let entry_row = EntryRow::builder()
            .title(row_title)
            .text(name)
            .show_apply_button(true)
            .input_purpose(purpose)
            .build();

        drop(desktop_file_borrow);

        let desktop_file_clone = self.desktop_file.clone();
        let toast_overlay_clone = self.toast_overlay.clone();
        let self_clone = self.clone();
        let desktop_file_key_clone = desktop_file_key.to_string();

        entry_row.connect_apply(move |entry_row| {
            let mut desktop_file_borrow = desktop_file_clone.borrow_mut();
            let undo_text = desktop_file_borrow
                .desktop_entry(&desktop_file_key_clone)
                .unwrap_or_default()
                .to_string();

            desktop_file_borrow
                .add_desktop_entry(desktop_file_key_clone.clone(), entry_row.text().to_string());
            drop(desktop_file_borrow);

            let saved_toast = Self::build_saved_toast();
            let desktop_file_clone = self_clone.desktop_file.clone();
            let self_clone_undo = self_clone.clone();
            let entry_row_clone = entry_row.clone();
            let desktop_file_key_undo_clone = desktop_file_key_clone.clone();

            saved_toast.connect_button_clicked(move |_| {
                entry_row_clone.set_text(&undo_text);
                let mut desktop_file_borrow = desktop_file_clone.borrow_mut();
                desktop_file_borrow.add_desktop_entry(
                    desktop_file_key_undo_clone.clone(),
                    entry_row_clone.text().to_string(),
                );
                drop(desktop_file_borrow);
                self_clone_undo.on_desktop_file_change();
            });
            toast_overlay_clone.add_toast(saved_toast);

            let desktop_file_clone = self_clone.desktop_file.clone();
            debug!(
                "Set a new '{}' on `desktop file`: {}",
                desktop_file_key_clone,
                &desktop_file_clone
                    .borrow()
                    .desktop_entry(&desktop_file_key_clone)
                    .unwrap_or_default()
            );

            self_clone.on_desktop_file_change();
        });

        entry_row
    }

    fn build_name_row(self: &Rc<Self>) -> EntryRow {
        let desktop_file_key = "Name";
        self.build_input_row("Web app name", InputPurpose::Name, desktop_file_key)
    }

    fn build_url_row(self: &Rc<Self>) -> EntryRow {
        let desktop_file_key = config::DesktopFile::URL_KEY;
        let validate_icon = Image::from_icon_name("dialog-warning-symbolic");
        let entry_row = self.build_input_row("Website URL", InputPurpose::Url, desktop_file_key);

        validate_icon.set_visible(false);
        validate_icon.set_css_classes(&["error"]);
        entry_row.add_suffix(&validate_icon);

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

        entry_row
    }

    fn build_browser_row(self: &Rc<Self>) -> ComboRow {
        let all_browsers = Rc::new(self.app.browsers_configs.get_all_browsers());

        // Some weird factory setup where the list calls factory methods...
        // First create all data structures, then set data from ListStore.
        // Why is this so unnecessary complicated? ¯\_(ツ)_/¯
        let list = gio::ListStore::new::<BoxedAnyObject>();
        for browser in all_browsers.iter() {
            let boxed = BoxedAnyObject::new(browser.clone());
            list.append(&boxed);
        }
        let factory = SignalListItemFactory::new();
        factory.connect_bind(|_, list_item| {
            let list_item = list_item.downcast_ref::<ListItem>().unwrap();
            let browser_item_boxed = list_item
                .item()
                .unwrap()
                .downcast::<BoxedAnyObject>()
                .unwrap();
            let browser = browser_item_boxed.borrow::<Rc<Browser>>();
            let box_container = gtk::Box::new(gtk::Orientation::Horizontal, 6);

            box_container.append(&browser.get_icon());
            box_container.append(&Label::new(Some(&browser.get_name_with_installation())));

            list_item.set_child(Some(&box_container));
        });

        let combo_row = ComboRow::builder()
            .title("Browser")
            .subtitle("Pick a browser")
            .model(&list)
            .factory(&factory)
            .build();

        let desktop_file_key = config::DesktopFile::BROWSER_ID;
        let desktop_file_clone = self.desktop_file.clone();
        let toast_overlay_clone = self.toast_overlay.clone();
        let self_clone = self.clone();
        let desktop_file_key_clone = desktop_file_key.to_string();
        let all_browsers_clone = all_browsers.clone();
        let is_blocked = Rc::new(Cell::new(false)); // ComboRow recurses on undo.

        combo_row.connect_selected_item_notify(move |combo_row| {
            if is_blocked.get() {
                return;
            }

            let selected_item = combo_row.selected_item();
            let Some(selected_item) = selected_item else {
                return;
            };
            let browser_item_boxed = selected_item.downcast::<BoxedAnyObject>().unwrap();
            let browser = browser_item_boxed.borrow::<Rc<Browser>>();
            let mut desktop_file_borrow = desktop_file_clone.borrow_mut();

            let undo_browser_id = desktop_file_borrow
                .desktop_entry(&desktop_file_key_clone.clone())
                .unwrap_or_default()
                .to_string();
            let undo_state = all_browsers_clone
                .iter()
                .position(|browser| browser.id == undo_browser_id);

            desktop_file_borrow
                .add_desktop_entry(desktop_file_key_clone.clone(), browser.id.clone());
            drop(desktop_file_borrow);

            let saved_toast = Self::build_saved_toast();
            let combo_row_clone = combo_row.clone();
            let desktop_file_clone = self_clone.desktop_file.clone();
            let desktop_file_key_undo_clone = desktop_file_key_clone.clone();
            let self_clone_undo = self_clone.clone();
            let is_blocked_clone = is_blocked.clone();

            saved_toast.connect_button_clicked(move |_| {
                is_blocked_clone.set(true);
                let Some(undo_state) = undo_state else {
                    return;
                };
                combo_row_clone.set_selected(undo_state.try_into().unwrap());
                let mut desktop_file_borrow = desktop_file_clone.borrow_mut();

                desktop_file_borrow.add_desktop_entry(
                    desktop_file_key_undo_clone.clone(),
                    undo_browser_id.clone(),
                );
                drop(desktop_file_borrow);

                self_clone_undo.on_desktop_file_change();
                is_blocked_clone.set(false);
            });
            toast_overlay_clone.add_toast(saved_toast);

            let desktop_file_clone = self_clone.desktop_file.clone();
            debug!(
                "Set a new '{}' on `desktop file`: {}",
                desktop_file_key_clone,
                &desktop_file_clone
                    .borrow()
                    .desktop_entry(&desktop_file_key_clone)
                    .unwrap_or_default()
            );

            self_clone.on_desktop_file_change();
        });

        combo_row
    }

    fn build_isolate_row(self: &Rc<Self>) -> SwitchRow {
        let desktop_file_key = config::DesktopFile::ISOLATE_KEY;
        let desktop_file_borrow = self.desktop_file.borrow();
        let is_isolated = desktop_file_borrow
            .desktop_entry(desktop_file_key)
            .is_some_and(|is_isolated| is_isolated == "true");

        let switch_row = SwitchRow::builder()
            .title("Isolate")
            .subtitle("Use an isolated profile")
            .active(is_isolated)
            .build();

        let desktop_file_clone = self.desktop_file.clone();
        let toast_overlay_clone = self.toast_overlay.clone();
        let self_clone = self.clone();
        let desktop_file_key_clone = desktop_file_key.to_string();
        let is_blocked = Rc::new(Cell::new(false)); // SwitchRow recurses on undo.

        switch_row.connect_active_notify(move |switch_row| {
            if is_blocked.get() {
                return;
            }

            let is_on = switch_row.is_active();
            let mut desktop_file_borrow = desktop_file_clone.borrow_mut();
            let undo_state = desktop_file_borrow
                .desktop_entry(&desktop_file_key_clone)
                .is_some_and(|is_isolated| is_isolated == "true");

            desktop_file_borrow
                .add_desktop_entry(desktop_file_key_clone.clone(), is_on.to_string());
            drop(desktop_file_borrow);

            let saved_toast = Self::build_saved_toast();
            let desktop_file_clone = self_clone.desktop_file.clone();
            let self_clone_undo = self_clone.clone();
            let switch_row_clone = switch_row.clone();
            let desktop_file_key_undo_clone = desktop_file_key_clone.clone();
            let is_blocked_clone = is_blocked.clone();

            saved_toast.connect_button_clicked(move |_| {
                is_blocked_clone.set(true);
                switch_row_clone.set_active(undo_state);
                let mut desktop_file_borrow = desktop_file_clone.borrow_mut();

                desktop_file_borrow.add_desktop_entry(
                    desktop_file_key_undo_clone.clone(),
                    switch_row_clone.is_active().to_string(),
                );
                drop(desktop_file_borrow);

                self_clone_undo.on_desktop_file_change();
                is_blocked_clone.set(false);
            });
            toast_overlay_clone.add_toast(saved_toast);

            let desktop_file_clone = self_clone.desktop_file.clone();
            debug!(
                "Set a new '{}' on `desktop file`: {}",
                desktop_file_key_clone,
                &desktop_file_clone
                    .borrow()
                    .desktop_entry(&desktop_file_key_clone)
                    .unwrap_or_default()
            );

            self_clone.on_desktop_file_change();
        });

        switch_row
    }

    fn build_update_icon_button(self: &Rc<Self>) -> Button {
        let button_content = ButtonContent::builder()
            .label("Update icon")
            .icon_name("software-update-available-symbolic")
            .build();
        let button = Button::builder().child(&button_content).build();

        let self_clone = self.clone();
        button.connect_clicked(move |_| {
            let desktop_file_borrow = self_clone.desktop_file.borrow();
            let undo_icon_path = desktop_file_borrow.icon().unwrap_or_default().to_string();

            let icon_picker = IconPicker::new(&self_clone.app, &self_clone.desktop_file);
            let dialog = icon_picker.show_dialog();

            let self_clone = self_clone.clone();
            dialog.connect_response(Some(IconPicker::DIALOG_SAVE), move |_, _| {
                let toast = Self::build_saved_toast();
                let undo_icon_path = undo_icon_path.clone();
                let self_clone_undo = self_clone.clone();

                toast.connect_button_clicked(move |_| {
                    let mut desktop_file_borrow = self_clone_undo.desktop_file.borrow_mut();
                    desktop_file_borrow
                        .add_desktop_entry("Icon".to_string(), undo_icon_path.clone());

                    drop(desktop_file_borrow);
                    self_clone_undo.on_desktop_file_change();
                });

                self_clone.on_desktop_file_change();
                self_clone.toast_overlay.add_toast(toast);
            });
        });

        button
    }

    fn build_saved_toast() -> Toast {
        let toast = Toast::new(Self::TOAST_SAVED);
        toast.set_button_label(Some(Self::TOAST_UNDO_BUTTON));
        toast.set_timeout(Self::TOAST_UNDO_TIMEOUT);

        toast
    }

    fn build_reset_toast() -> Toast {
        let toast = Toast::new(Self::TOAST_RESET);
        toast.set_timeout(Self::TOAST_MESSAGE_TIMEOUT);

        toast
    }

    fn reset_reset_button(&self) {
        if self.desktop_file_original.to_string() == self.desktop_file.borrow().to_string() {
            self.reset_button.set_sensitive(false);
        } else {
            self.reset_button.set_sensitive(true);
        }
    }

    fn reset_app_header(&self) {
        debug!("Resetting app header");

        let mut pref_groups = self.pref_groups.borrow_mut();

        for pref_group in pref_groups.iter() {
            self.prefs_page.remove(pref_group);
        }

        let Some(old_app_header) = pref_groups.first_mut() else {
            return;
        };
        let new_app_header = self.build_app_header();
        *old_app_header = new_app_header;

        for pref_group in pref_groups.iter() {
            self.prefs_page.add(pref_group);
        }
    }

    fn reset_view(self: &Rc<Self>) {
        debug!("Resetting view");
        self.init();
        self.reset_reset_button();
    }

    fn on_desktop_file_change(&self) {
        debug!("Desktop file changed");

        self.reset_reset_button();
        self.reset_app_header();
    }
}
