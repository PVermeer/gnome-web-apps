mod icon_picker;

use crate::{
    application::{
        App,
        pages::{NavPage, PrefPage},
    },
    ext::desktop_entry::{self, DesktopEntryExt},
    services::browsers::Browser,
};
use freedesktop_desktop_entry::DesktopEntry;
use gtk::{
    ListItem, SignalListItemFactory, gio,
    glib::{self, BoxedAnyObject, object::Cast},
    prelude::ListItemExt,
};
use icon_picker::IconPicker;
use libadwaita::{
    ActionRow, ButtonContent, ComboRow, EntryRow, HeaderBar, NavigationPage, PreferencesGroup,
    PreferencesPage, SwitchRow, Toast, ToastOverlay, WrapBox,
    gtk::{
        self, Button, Image, InputPurpose, Label, Orientation,
        prelude::{BoxExt, ButtonExt, EditableExt, WidgetExt},
    },
    prelude::{
        ComboRowExt, EntryRowExt, NavigationPageExt, PreferencesGroupExt, PreferencesPageExt,
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
    is_new: RefCell<bool>,
    browser_can_isolate: RefCell<bool>,
    nav_page: NavigationPage,
    app: Rc<App>,
    header: HeaderBar,
    desktop_file: Rc<RefCell<DesktopEntry>>,
    desktop_file_original: DesktopEntry,
    prefs_page: PreferencesPage,
    pref_groups: RefCell<Vec<PreferencesGroup>>,
    toast_overlay: ToastOverlay,
    reset_button: Button,
    change_icon_button: Button,
    run_app_button: Button,
    save_button: Button,
    name_row: EntryRow,
    url_row: EntryRow,
    isolate_row: SwitchRow,
    browser_row: ComboRow,
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
    const TOAST_MESSAGE_TIMEOUT: u32 = 1;
    const TOAST_UNDO_TIMEOUT: u32 = 2;
    const TOAST_SAVED: &str = "Saved";
    const TOAST_RESET: &str = "Reset";
    const TOAST_UNDO_BUTTON: &str = "Undo";

    pub fn new(app: &Rc<App>, desktop_file: &Rc<RefCell<DesktopEntry>>, is_new: bool) -> Rc<Self> {
        let desktop_file_borrow = desktop_file.borrow();
        let desktop_file_original = desktop_file_borrow.clone(); // Deep clone
        let title = desktop_file_borrow
            .name(&app.desktop_file_locales)
            .and_then(|name| if name.is_empty() { None } else { Some(name) })
            .unwrap_or(Cow::Borrowed("New Web App"));
        let browser_can_isolate = desktop_file_borrow
            .desktop_entry(&desktop_entry::KeysExt::BrowserId.to_string())
            .and_then(|browser_id| app.browsers_configs.get_by_id(browser_id))
            .is_some_and(|browser| browser.can_isolate);
        let icon = "preferences-desktop-apps-symbolic";
        let PrefPage {
            nav_page,
            prefs_page,
            toast_overlay,
            header,
            ..
        } = Self::build_nav_page(&title, icon).with_preference_page();
        drop(desktop_file_borrow);

        let reset_button = Self::build_header_reset_button();
        let change_icon_button = Self::build_change_icon_button();
        let run_app_button = Self::build_run_app_button();
        let save_button = Self::build_save_button();
        let name_row = Self::build_name_row(desktop_file);
        let url_row = Self::build_url_row(desktop_file);
        let isolate_row = Self::build_isolate_row(desktop_file, browser_can_isolate);
        let browser_row = Self::build_browser_row(app, desktop_file);

        Rc::new(Self {
            is_new: RefCell::new(is_new),
            browser_can_isolate: RefCell::new(browser_can_isolate),
            nav_page,
            app: app.clone(),
            header,
            desktop_file: desktop_file.clone(),
            desktop_file_original,
            prefs_page,
            pref_groups: RefCell::new(Vec::new()),
            toast_overlay,
            reset_button,
            change_icon_button,
            run_app_button,
            save_button,
            name_row,
            url_row,
            isolate_row,
            browser_row,
        })
    }

    pub fn init(self: &Rc<Self>) {
        let self_clone = self.clone();

        self.header.pack_end(&self.reset_button);
        self.reset_button
            .connect_clicked(move |_| self_clone.reset_desktop_file());
        let web_app_header = self.build_app_header();
        let general_pref_group = self.build_general_pref_group();
        let save_footer = self.build_save_footer();

        let mut pref_groups = self.pref_groups.borrow_mut();
        pref_groups.push(web_app_header);
        pref_groups.push(general_pref_group);
        if *self.is_new.borrow() {
            self.run_app_button.set_visible(false);
            pref_groups.push(save_footer);
        }

        for pref_group in pref_groups.iter() {
            self.prefs_page.add(pref_group);
        }
        drop(pref_groups);

        self.connect_change_icon_button();
        self.connect_run_button();
        self.connect_save_button();
    }

    fn reset_desktop_file(self: &Rc<Self>) {
        debug!("Resetting desktop file");

        let browsers = self.app.browsers_configs.get_all_browsers();
        let mut desktop_file_borrow = self.desktop_file.borrow_mut();
        let save_path = desktop_file_borrow.path.clone();
        *desktop_file_borrow = self.desktop_file_original.clone();
        desktop_file_borrow.path = save_path;

        let name = desktop_file_borrow
            .desktop_entry("Name")
            .unwrap_or_default()
            .to_string();
        let url = desktop_file_borrow
            .desktop_entry(&desktop_entry::KeysExt::Url.to_string())
            .unwrap_or_default()
            .to_string();
        let is_isolated = desktop_file_borrow
            .desktop_entry(&desktop_entry::KeysExt::Isolate.to_string())
            .unwrap_or_default()
            .eq("true");
        let browser_index = desktop_file_borrow
            .desktop_entry(&desktop_entry::KeysExt::BrowserId.to_string())
            .and_then(|browser_id| browsers.iter().position(|browser| browser.id == browser_id))
            .and_then(|index| index.try_into().ok())
            .unwrap_or(0);

        drop(desktop_file_borrow);

        self.name_row.set_text(&name);
        self.url_row.set_text(&url);
        self.isolate_row.set_active(is_isolated);
        self.browser_row.set_selected(browser_index);

        self.on_desktop_file_change();

        let toast = Self::build_reset_toast();
        self.toast_overlay.add_toast(toast);
    }

    fn build_header_reset_button() -> Button {
        let reset_button = Button::with_label("Reset");
        reset_button.set_sensitive(false);

        reset_button
    }

    fn build_run_app_button() -> Button {
        Button::builder()
            .label("Open")
            .css_classes(["suggested-action", "pill"])
            .build()
    }

    fn build_app_header(&self) -> PreferencesGroup {
        let desktop_file_borrow = self.desktop_file.borrow_mut();

        let pref_group = PreferencesGroup::builder().build();
        let content_box = gtk::Box::new(Orientation::Vertical, 6);
        let app_name = desktop_file_borrow
            .name(&self.app.desktop_file_locales)
            .unwrap_or(Cow::Borrowed("No name..."));
        let app_label = Label::builder()
            .label(app_name)
            .css_classes(["title-1"])
            .build();

        if desktop_file_borrow.exec().is_some() {
            self.run_app_button.set_sensitive(true);
        } else {
            self.run_app_button.set_sensitive(false);
        }

        let button_wrap_box = WrapBox::builder()
            .align(0.5)
            .margin_top(12)
            .margin_bottom(12)
            .build();
        button_wrap_box.append(&self.run_app_button);

        let app_image = desktop_file_borrow.get_image_icon();
        app_image.set_css_classes(&["icon-dropshadow"]);
        app_image.set_pixel_size(96);
        app_image.set_margin_start(25);
        app_image.set_margin_end(25);

        let browser_label = Label::new(None);
        let browser_id = desktop_file_borrow
            .desktop_entry(&desktop_entry::KeysExt::BrowserId.to_string())
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
        let pref_group = PreferencesGroup::builder()
            .header_suffix(&self.change_icon_button)
            .build();

        pref_group.add(&self.name_row);
        pref_group.add(&self.url_row);
        pref_group.add(&self.isolate_row);
        pref_group.add(&self.browser_row);

        self.connect_name_row();
        self.connect_url_row();
        self.connect_isolate_row();
        self.connect_browser_row();

        pref_group
    }

    fn build_input_row(
        row_title: &str,
        purpose: InputPurpose,
        desktop_file: &Rc<RefCell<DesktopEntry>>,
        desktop_file_key: &str,
    ) -> EntryRow {
        let desktop_file_borrow = desktop_file.borrow();
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

        entry_row
    }

    fn build_name_row(desktop_file: &Rc<RefCell<DesktopEntry>>) -> EntryRow {
        Self::build_input_row("Web app name", InputPurpose::Name, desktop_file, "Name")
    }

    fn build_url_row(desktop_file: &Rc<RefCell<DesktopEntry>>) -> EntryRow {
        Self::build_input_row(
            "Website URL",
            InputPurpose::Url,
            desktop_file,
            &desktop_entry::KeysExt::Url.to_string(),
        )
    }

    fn build_isolate_row(
        desktop_file: &Rc<RefCell<DesktopEntry>>,
        browser_can_isolate: bool,
    ) -> SwitchRow {
        let desktop_file_key = desktop_entry::KeysExt::Isolate.to_string();
        let mut desktop_file_borrow = desktop_file.borrow_mut();

        let is_isolated = desktop_file_borrow
            .desktop_entry(&desktop_file_key)
            .is_some_and(|is_isolated| is_isolated == "true");

        let switch_row = SwitchRow::builder()
            .title("Isolate")
            .subtitle("Use an isolated profile")
            .active(is_isolated)
            .sensitive(browser_can_isolate)
            .build();

        if !browser_can_isolate && is_isolated {
            debug!("Found desktop file with isolate on a browser that is incapable");
            switch_row.set_active(false);
            desktop_file_borrow.add_desktop_entry(desktop_file_key.clone(), false.to_string());
        }

        // SwitchRow has already a setting on load, so save this.
        desktop_file_borrow
            .add_desktop_entry(desktop_file_key.clone(), switch_row.is_active().to_string());

        switch_row
    }

    fn build_browser_row(app: &Rc<App>, desktop_file: &Rc<RefCell<DesktopEntry>>) -> ComboRow {
        let all_browsers = app.browsers_configs.get_all_browsers();
        let desktop_file_key = desktop_entry::KeysExt::BrowserId.to_string();

        // Some weird factory setup where the list calls factory methods...
        // First create all data structures, then set data from ListStore.
        // Why is this so unnecessary complicated? ¯\_(ツ)_/¯
        let list = gio::ListStore::new::<BoxedAnyObject>();
        for browser in &all_browsers {
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

        if let Some(browser_id) = desktop_file.borrow().desktop_entry(&desktop_file_key) {
            let browser_id = browser_id.to_string();
            let index = all_browsers
                .iter()
                .position(|browser| browser.id == browser_id);
            if let Some(index) = index {
                combo_row.set_selected(index.try_into().unwrap());
            }
        // ComboRow has already a selected item on load, so save this if empty.
        } else if let Some(browser) = all_browsers.first() {
            desktop_file
                .borrow_mut()
                .add_desktop_entry(desktop_file_key.clone(), browser.id.clone());
        }

        combo_row
    }

    fn build_change_icon_button() -> Button {
        let button_content = ButtonContent::builder()
            .label("Change icon")
            .icon_name("software-update-available-symbolic")
            .build();

        Button::builder().child(&button_content).build()
    }

    fn build_save_footer(self: &Rc<Self>) -> PreferencesGroup {
        let pref_group = PreferencesGroup::builder().build();
        let content_box = gtk::Box::new(Orientation::Vertical, 6);
        let save_button = &self.save_button;
        save_button.set_sensitive(false);

        let button_wrap_box = WrapBox::builder()
            .align(0.5)
            .margin_top(12)
            .margin_bottom(12)
            .build();
        button_wrap_box.append(save_button);

        content_box.append(&button_wrap_box);

        pref_group.add(&content_box);

        pref_group
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

    fn build_error_toast(message: &str) -> Toast {
        let toast = Toast::new(message);
        toast.set_timeout(Self::TOAST_MESSAGE_TIMEOUT);

        toast
    }

    fn build_save_button() -> Button {
        Button::builder()
            .label("Save")
            .css_classes(["suggested-action", "pill"])
            .build()
    }

    fn connect_change_icon_button(self: &Rc<Self>) {
        if *self.is_new.borrow() {
            self.change_icon_button.set_sensitive(false);
        }

        let self_clone = self.clone();
        self.change_icon_button.connect_clicked(move |_| {
            let desktop_file_borrow = self_clone.desktop_file.borrow();
            let undo_icon_path = desktop_file_borrow.icon().unwrap_or_default().to_string();
            let undo_icon_path_success = undo_icon_path.clone();
            let undo_icon_path_fail = undo_icon_path.clone();

            let self_clone_success = self_clone.clone();
            let self_clone_fail = self_clone.clone();

            let icon_picker = IconPicker::new(&self_clone.app, &self_clone.desktop_file);

            drop(desktop_file_borrow);

            icon_picker.show_dialog(
                Some(move || {
                    // Success
                    let toast = Self::build_saved_toast();
                    let undo_icon_path = undo_icon_path_success.clone();
                    let self_clone_undo = self_clone_success.clone();

                    toast.connect_button_clicked(move |_| {
                        let mut desktop_file_borrow = self_clone_undo.desktop_file.borrow_mut();
                        desktop_file_borrow
                            .add_desktop_entry("Icon".to_string(), undo_icon_path.clone());

                        drop(desktop_file_borrow);
                        self_clone_undo.on_desktop_file_change();
                    });

                    self_clone_success.on_desktop_file_change();
                    self_clone_success.toast_overlay.add_toast(toast);
                }),
                Some(move || {
                    // Fail
                    let toast = Self::build_error_toast("Failed to save icon");
                    let undo_icon_path = undo_icon_path_fail.clone();
                    self_clone_fail
                        .desktop_file
                        .borrow_mut()
                        .add_desktop_entry("Icon".to_string(), undo_icon_path);

                    self_clone_fail.on_desktop_file_change();
                    self_clone_fail.toast_overlay.add_toast(toast);
                }),
            );
        });
    }

    fn connect_run_button(self: &Rc<Self>) {
        let self_clone = self.clone();

        self.run_app_button.connect_clicked(move |_| {
            let desktop_file_borrow = self_clone.desktop_file.borrow();
            if let Some(exec) = desktop_file_borrow.exec() {
                let executable = exec.to_string();
                debug!("Running web app: '{executable}'");

                #[allow(clippy::zombie_processes)]
                let result = Command::new("sh").arg("-c").arg(executable.clone()).spawn();

                if let Err(error) = result {
                    error!("Failed to run app '{executable}': {error:?}");
                }
            }
        });
    }

    fn connect_save_button(self: &Rc<Self>) {
        let self_clone = self.clone();

        self.save_button.connect_clicked(move |_| {
            self_clone.on_new_desktop_file_save();
        });
    }

    fn connect_input_row(self: &Rc<Self>, entry_row: &EntryRow, desktop_file_key: &str) {
        let self_clone = self.clone();
        let desktop_file_key_clone = desktop_file_key.to_string();

        entry_row.connect_apply(move |entry_row| {
            let mut desktop_file_borrow = self_clone.desktop_file.borrow_mut();
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
            self_clone.toast_overlay.add_toast(saved_toast);

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

        if desktop_file_key == "Name" {
            let self_clone = self.clone();
            entry_row.connect_apply(move |entry_row| {
                let title = entry_row.text();
                if title.is_empty() {
                    return;
                }
                self_clone.nav_page.set_title(&title);
            });
        }
    }

    fn connect_name_row(self: &Rc<Self>) {
        self.connect_input_row(&self.name_row, "Name");
    }

    fn connect_url_row(self: &Rc<Self>) {
        let self_clone = self.clone();

        let validate_icon = Image::from_icon_name("dialog-warning-symbolic");
        validate_icon.set_visible(false);
        validate_icon.set_css_classes(&["error"]);
        self.url_row.add_suffix(&validate_icon);

        self.url_row.connect_changed(move |entry_row| {
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
                self_clone.change_icon_button.set_sensitive(false);
            }
        });

        let self_clone = self.clone();

        self.url_row.connect_apply(move |entry_row| {
            self_clone.change_icon_button.set_sensitive(true);

            let url = entry_row.text().to_string();
            let self_clone = self_clone.clone();

            glib::spawn_future_local(async move {
                let icon_picker = IconPicker::new(&self_clone.app, &self_clone.desktop_file);

                if let Err(error) = icon_picker.set_first_icon(&url).await {
                    self_clone
                        .desktop_file
                        .borrow_mut()
                        .add_desktop_entry("Icon".to_string(), String::new());
                    error!("{error:?}");
                }
                self_clone.on_desktop_file_change();
            });
        });

        self.connect_input_row(&self.url_row, &desktop_entry::KeysExt::Url.to_string());
    }

    fn connect_isolate_row(self: &Rc<Self>) {
        let is_blocked = Rc::new(Cell::new(false)); // SwitchRow recurses on undo.
        let self_clone = self.clone();

        self.isolate_row.connect_active_notify(move |switch_row| {
            if is_blocked.get() {
                return;
            }
            let desktop_file_key = desktop_entry::KeysExt::Isolate.to_string();
            let is_on = switch_row.is_active();
            let mut desktop_file_borrow = self_clone.desktop_file.borrow_mut();
            let undo_state = desktop_file_borrow
                .desktop_entry(&desktop_file_key)
                .is_some_and(|is_isolated| is_isolated == "true");

            desktop_file_borrow.add_desktop_entry(desktop_file_key.clone(), is_on.to_string());
            drop(desktop_file_borrow);

            let saved_toast = Self::build_saved_toast();
            let desktop_file_clone = self_clone.desktop_file.clone();
            let self_clone_undo = self_clone.clone();
            let switch_row_clone = switch_row.clone();
            let desktop_file_key_undo_clone = desktop_file_key.clone();
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
            self_clone.toast_overlay.add_toast(saved_toast);

            let desktop_file_clone = self_clone.desktop_file.clone();
            debug!(
                "Set a new '{}' on `desktop file`: {}",
                desktop_file_key,
                &desktop_file_clone
                    .borrow()
                    .desktop_entry(&desktop_file_key)
                    .unwrap_or_default()
            );

            self_clone.on_desktop_file_change();
        });
    }

    fn connect_browser_row(self: &Rc<Self>) {
        let all_browsers = self.app.browsers_configs.get_all_browsers();
        let desktop_file_key = desktop_entry::KeysExt::BrowserId.to_string();
        let desktop_file_clone = self.desktop_file.clone();
        let toast_overlay_clone = self.toast_overlay.clone();
        let self_clone = self.clone();
        let all_browsers_clone = all_browsers.clone();
        let is_blocked = Rc::new(Cell::new(false)); // ComboRow recurses on undo.

        self.browser_row
            .connect_selected_item_notify(move |combo_row| {
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
                    .desktop_entry(&desktop_file_key)
                    .unwrap_or_default()
                    .to_string();
                let undo_state = all_browsers_clone
                    .iter()
                    .position(|browser| browser.id == undo_browser_id);

                desktop_file_borrow.add_desktop_entry(desktop_file_key.clone(), browser.id.clone());
                drop(desktop_file_borrow);

                *self_clone.browser_can_isolate.borrow_mut() = browser.can_isolate;

                let saved_toast = Self::build_saved_toast();
                let combo_row_clone = combo_row.clone();
                let desktop_file_clone = self_clone.desktop_file.clone();
                let desktop_file_key_undo_clone = desktop_file_key.clone();
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
                    desktop_file_key,
                    &desktop_file_clone
                        .borrow()
                        .desktop_entry(&desktop_file_key)
                        .unwrap_or_default()
                );

                self_clone.on_desktop_file_change();
            });
    }

    fn reset_reset_button(self: &Rc<Self>) {
        if self.desktop_file_original.to_string() == self.desktop_file.borrow().to_string() {
            self.reset_button.set_sensitive(false);
        } else {
            self.reset_button.set_sensitive(true);
        }
    }

    fn reset_app_header(self: &Rc<Self>) {
        debug!("Resetting app header");

        let mut pref_groups = self.pref_groups.borrow_mut();
        if pref_groups.is_empty() {
            return;
        }

        for pref_group in pref_groups.iter() {
            self.prefs_page.remove(pref_group);
        }

        // Pretty ugly but the old header needs to be dropped before creating a new one
        let old_app_header = pref_groups.remove(0);
        drop(old_app_header);
        let new_app_header = self.build_app_header();
        pref_groups.insert(0, new_app_header);

        for pref_group in pref_groups.iter() {
            self.prefs_page.add(pref_group);
        }
    }

    fn reset_browser_isolation(self: &Rc<Self>) {
        let browser_can_isolate = *self.browser_can_isolate.borrow();
        self.isolate_row.set_sensitive(browser_can_isolate);
        if !browser_can_isolate {
            self.isolate_row.set_active(false);
        }
    }

    fn on_desktop_file_change(self: &Rc<Self>) {
        debug!("Desktop file changed");

        let is_new = *self.is_new.borrow();

        if is_new && self.desktop_file.borrow().validate(&self.app).is_ok() {
            self.save_button.set_sensitive(true);
        } else {
            self.save_button.set_sensitive(false);
        }

        if !is_new && self.desktop_file.borrow_mut().save(&self.app).is_err() {
            let toast = Self::build_error_toast("Failed to save app");
            self.toast_overlay.add_toast(toast);
        }

        self.reset_reset_button();
        self.reset_app_header();
        self.reset_browser_isolation();
    }

    fn on_new_desktop_file_save(self: &Rc<Self>) {
        if let Err(error) = self.desktop_file.borrow().validate(&self.app) {
            error!("Invalid desktop file to save: '{error}'");
            let toast = Self::build_error_toast("Failed to save app");
            self.toast_overlay.add_toast(toast);
            return;
        }
        *self.is_new.borrow_mut() = false;
        self.run_app_button.set_visible(true);
        self.save_button.set_visible(false);
        self.on_desktop_file_change();
    }
}
