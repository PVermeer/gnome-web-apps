mod icon_picker;

use crate::application::{
    App,
    pages::{NavPage, PrefPage},
};
use common::{
    browsers::{Base, Browser},
    desktop_file::{DesktopFile, DesktopFileError},
    utils,
};
use gtk::{
    Align, EventControllerMotion, ListItem, SignalListItemFactory, gio,
    glib::{self, BoxedAnyObject, object::Cast},
    prelude::ListItemExt,
};
use icon_picker::IconPicker;
use libadwaita::{
    ActionRow, ButtonContent, ComboRow, EntryRow, HeaderBar, NavigationPage, NavigationView,
    PreferencesGroup, PreferencesPage, Spinner, SwitchRow, Toast, ToastOverlay, ToastPriority,
    WrapBox,
    gtk::{
        self, Button, Image, InputPurpose, Label, Orientation,
        prelude::{BoxExt, ButtonExt, EditableExt, WidgetExt},
    },
    prelude::{
        ComboRowExt, EntryRowExt, NavigationPageExt, PreferencesGroupExt, PreferencesPageExt,
        PreferencesRowExt,
    },
};
use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
};
use std::{fmt::Write as _, fs};
use tracing::{debug, error};
use validator::ValidateUrl;

pub struct WebAppView {
    is_new: RefCell<bool>,
    nav_page: NavigationPage,
    nav_view: Rc<NavigationView>,
    app: Rc<App>,
    header: HeaderBar,
    desktop_file: Rc<RefCell<DesktopFile>>,
    desktop_file_original: DesktopFile,
    prefs_page: PreferencesPage,
    pref_groups: RefCell<Vec<PreferencesGroup>>,
    toast_overlay: ToastOverlay,
    reset_button: Button,
    change_icon_button: Button,
    run_app_button: Button,
    save_button: Button,
    delete_button: Button,
    name_row: EntryRow,
    url_row: EntryRow,
    isolate_row: SwitchRow,
    maximize_row: SwitchRow,
    browser_row: ComboRow,
    icon_picker: RefCell<Option<Rc<IconPicker>>>,
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
    const TOAST_MESSAGE_TIMEOUT: u32 = 4;
    const TOAST_RESET: &str = "Reset";

    pub fn new(
        app: &Rc<App>,
        nav_view: &Rc<NavigationView>,
        desktop_file: &Rc<RefCell<DesktopFile>>,
        is_new: bool,
    ) -> Rc<Self> {
        let desktop_file_borrow = desktop_file.borrow();
        let desktop_file_original = desktop_file_borrow.clone(); // Deep clone
        let title = desktop_file_borrow
            .get_name()
            .unwrap_or("New Web App".to_string());
        let browser_can_isolate = desktop_file_borrow
            .get_browser()
            .is_some_and(|browser| browser.can_isolate);
        let browser_can_maximize = desktop_file_borrow
            .get_browser()
            .is_some_and(|browser| browser.can_start_maximized);
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
        let run_app_button = Self::build_run_app_button(is_new);
        let save_button = Self::build_save_button(is_new);
        let delete_button = Self::build_delete_button();
        let name_row = Self::build_name_row(desktop_file);
        let url_row = Self::build_url_row(desktop_file);
        let isolate_row = Self::build_isolate_row(desktop_file, browser_can_isolate);
        let maximize_row = Self::build_maximize_row(desktop_file, browser_can_maximize);
        let browser_row = Self::build_browser_row(app, desktop_file);

        Rc::new(Self {
            is_new: RefCell::new(is_new),
            nav_page,
            nav_view: nav_view.clone(),
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
            delete_button,
            name_row,
            url_row,
            isolate_row,
            maximize_row,
            browser_row,
            icon_picker: RefCell::new(None),
        })
    }

    pub fn init(self: &Rc<Self>) {
        let self_clone = self.clone();

        self.header.pack_end(&self.reset_button);
        self.reset_button
            .connect_clicked(move |_| self_clone.reset_desktop_file());
        let web_app_header = self.build_app_header();
        let general_pref_group = self.build_general_pref_group();
        let button_footer = self.build_button_footer();

        let mut pref_groups_borrow = self.pref_groups.borrow_mut();
        pref_groups_borrow.push(web_app_header);
        pref_groups_borrow.push(general_pref_group);
        pref_groups_borrow.push(button_footer);

        for pref_group in pref_groups_borrow.iter() {
            self.prefs_page.add(pref_group);
        }
        drop(pref_groups_borrow);

        self.connect_change_icon_button();
        self.connect_run_app_button();
    }

    pub fn get_is_new(self: &Rc<Self>) -> bool {
        *self.is_new.borrow()
    }

    pub fn get_icon_picker(self: &Rc<Self>) -> Rc<IconPicker> {
        if let Some(icon_picker) = self.icon_picker.borrow().clone() {
            icon_picker
        } else {
            let icon_picker = IconPicker::new(&self.app, &self.desktop_file);
            *self.icon_picker.borrow_mut() = Some(icon_picker.clone());
            icon_picker
        }
    }

    fn reset_icon_picker(self: &Rc<Self>) {
        *self.icon_picker.borrow_mut() = None;
    }

    fn reset_desktop_file(self: &Rc<Self>) {
        debug!("Resetting desktop file");

        let mut desktop_file_borrow = self.desktop_file.borrow_mut();
        let save_path = desktop_file_borrow.get_path();
        *desktop_file_borrow = self.desktop_file_original.clone();
        desktop_file_borrow.set_path(&save_path);

        let name = desktop_file_borrow.get_name().unwrap_or_default();
        let url = desktop_file_borrow.get_url().unwrap_or_default();
        let is_isolated = desktop_file_borrow.get_isolated().unwrap_or(false);
        let browser_index = desktop_file_borrow
            .get_browser()
            .and_then(|browser| browser.get_index())
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

    fn build_app_header(&self) -> PreferencesGroup {
        let desktop_file_borrow = self.desktop_file.borrow_mut();

        let pref_group = PreferencesGroup::builder().build();
        let content_box = gtk::Box::new(Orientation::Vertical, 6);
        let app_name = desktop_file_borrow
            .get_name()
            .unwrap_or("No name...".to_string());
        let app_label = Label::builder()
            .label(app_name)
            .css_classes(["title-1"])
            .build();

        if desktop_file_borrow.get_exec().is_some() {
            self.run_app_button.set_sensitive(true);
        } else {
            self.run_app_button.set_sensitive(false);
        }

        let button_wrap_box = WrapBox::builder()
            .align(0.5)
            .margin_top(12)
            .margin_bottom(12)
            .build();

        // For some reason the button still has a parent sometimes...
        if let Some(parent) = self.run_app_button.parent()
            && let Some(wrap_box) = parent.downcast_ref::<WrapBox>()
        {
            wrap_box.remove(&self.run_app_button);
        }

        button_wrap_box.append(&self.run_app_button);

        let app_image = desktop_file_borrow.get_icon();
        app_image.add_css_class("icon-dropshadow");
        app_image.set_pixel_size(96);
        app_image.set_margin_start(25);
        app_image.set_margin_end(25);

        let browser_label = Label::new(None);
        if let Some(browser) = desktop_file_borrow.get_browser() {
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
        pref_group.add(&self.maximize_row);
        pref_group.add(&self.browser_row);

        self.connect_name_row();
        self.connect_url_row();
        self.connect_isolate_row();
        self.connect_maximize_row();
        self.connect_browser_row();

        pref_group
    }

    fn build_name_row(desktop_file: &Rc<RefCell<DesktopFile>>) -> EntryRow {
        let name = desktop_file.borrow().get_name().unwrap_or_default();

        EntryRow::builder()
            .title("Web app name")
            .text(name)
            .show_apply_button(true)
            .input_purpose(InputPurpose::Name)
            .build()
    }

    fn build_url_row(desktop_file: &Rc<RefCell<DesktopFile>>) -> EntryRow {
        let url = desktop_file.borrow().get_url().unwrap_or_default();

        EntryRow::builder()
            .title("Website URL")
            .text(url)
            .show_apply_button(true)
            .input_purpose(InputPurpose::Url)
            .build()
    }

    fn build_isolate_row(
        desktop_file: &Rc<RefCell<DesktopFile>>,
        browser_can_isolate: bool,
    ) -> SwitchRow {
        let mut desktop_file_borrow = desktop_file.borrow_mut();
        let has_isolated = desktop_file_borrow.get_isolated();
        let is_isolated = has_isolated.unwrap_or(false);

        let switch_row = SwitchRow::builder()
            .title("Isolate")
            .subtitle("Use an isolated profile")
            .active(is_isolated)
            .sensitive(browser_can_isolate)
            .build();

        if !browser_can_isolate && is_isolated {
            debug!("Found desktop file with isolate on a browser that is incapable");
            switch_row.set_active(false);
        }

        // SwitchRow has already a setting on load, so sync this if empty
        if has_isolated.is_none() {
            desktop_file_borrow.set_isolated(switch_row.is_active());
        }

        switch_row
    }

    fn build_maximize_row(
        desktop_file: &Rc<RefCell<DesktopFile>>,
        browser_can_maximize: bool,
    ) -> SwitchRow {
        let mut desktop_file_borrow = desktop_file.borrow_mut();
        let has_maximized = desktop_file_borrow.get_maximized();
        let is_maximized = has_maximized.unwrap_or(false);

        let switch_row = SwitchRow::builder()
            .title("Maximize")
            .subtitle("Always start the app maximized")
            .active(is_maximized)
            .sensitive(browser_can_maximize)
            .build();

        if !browser_can_maximize && is_maximized {
            debug!("Found desktop file with maximize on a browser that is incapable");
            switch_row.set_active(false);
        }

        // SwitchRow has already a setting on load, so sync this if empty
        if has_maximized.is_none() {
            desktop_file_borrow.set_maximized(switch_row.is_active());
        }

        switch_row
    }

    fn build_browser_row(app: &Rc<App>, desktop_file: &Rc<RefCell<DesktopFile>>) -> ComboRow {
        let all_browsers = app.browser_configs.get_all_browsers();

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
            let icon = browser.get_icon();

            box_container.append(&icon);
            box_container.append(&Label::new(Some(&browser.get_name_with_installation())));

            if browser.is_not_installed() {
                icon.add_css_class("error");
                box_container.add_css_class("dimmed");
                list_item.set_activatable(false);
                list_item.set_selectable(false);
            }

            list_item.set_child(Some(&box_container));
        });

        let combo_row = ComboRow::builder()
            .title("Browser")
            .subtitle("Pick a browser")
            .model(&list)
            .factory(&factory)
            .build();

        if let Some(browser_index) = desktop_file
            .borrow()
            .get_browser()
            .and_then(|browser| browser.get_index())
        {
            combo_row.set_selected(browser_index.try_into().unwrap());
        } else if let Some(browser) = all_browsers.first() {
            // ComboRow has already a selected item on load, so sync this if empty.
            desktop_file.borrow_mut().set_browser(browser);
        }

        combo_row
    }

    fn build_button_footer(self: &Rc<Self>) -> PreferencesGroup {
        fn button_wrap_box(button: &Button) -> WrapBox {
            let wrapbox = WrapBox::builder()
                .align(0.5)
                .margin_start(12)
                .margin_end(12)
                .build();
            wrapbox.append(button);
            wrapbox
        }

        let pref_group = PreferencesGroup::new();
        let content_box = gtk::Box::builder()
            .orientation(Orientation::Horizontal)
            .halign(Align::Center)
            .margin_top(12)
            .margin_bottom(12)
            .build();

        content_box.append(&button_wrap_box(&self.save_button));
        content_box.append(&button_wrap_box(&self.delete_button));
        pref_group.add(&content_box);

        self.connect_save_button();
        self.connect_delete_button();

        pref_group
    }

    fn build_reset_toast() -> Toast {
        let toast = Toast::new(Self::TOAST_RESET);
        toast.set_timeout(Self::TOAST_MESSAGE_TIMEOUT);

        toast
    }

    fn build_change_icon_button() -> Button {
        let button_content = ButtonContent::builder()
            .label("Change icon")
            .icon_name("software-update-available-symbolic")
            .build();

        Button::builder().child(&button_content).build()
    }

    fn build_run_app_button(is_new: bool) -> Button {
        Button::builder()
            .label("Open")
            .css_classes(["suggested-action", "pill"])
            .visible(!is_new)
            .build()
    }

    fn build_save_button(is_new: bool) -> Button {
        Button::builder()
            .label("Save")
            .css_classes(["suggested-action", "pill"])
            .visible(is_new)
            .sensitive(false)
            .build()
    }

    fn build_delete_button() -> Button {
        let button = Button::builder()
            .label("Delete")
            .css_classes(["destructive-action", "pill", "dimmed"])
            .build();

        let controller = EventControllerMotion::new();
        let button_clone = button.clone();
        controller.connect_enter(move |_, _, _| {
            button_clone.remove_css_class("dimmed");
        });
        let button_clone = button.clone();
        controller.connect_leave(move |_| {
            button_clone.add_css_class("dimmed");
        });

        button.add_controller(controller);

        button
    }

    fn build_validate_icon() -> Image {
        let validate_icon = Image::from_icon_name("dialog-warning-symbolic");
        validate_icon.set_visible(false);
        validate_icon.set_css_classes(&["error"]);

        validate_icon
    }

    fn connect_change_icon_button(self: &Rc<Self>) {
        if *self.is_new.borrow() {
            self.change_icon_button.set_sensitive(false);
        }

        let self_clone = self.clone();
        self.change_icon_button.connect_clicked(move |_| {
            let desktop_file_borrow = self_clone.desktop_file.borrow();
            let undo_icon_path = desktop_file_borrow
                .get_icon_path()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let undo_icon_path_fail = undo_icon_path.clone();

            let self_clone_success = self_clone.clone();
            let self_clone_fail = self_clone.clone();

            drop(desktop_file_borrow);

            let icon_picker = self_clone.get_icon_picker();

            icon_picker.show_dialog(
                Some(move || {
                    // Success
                    self_clone_success.on_desktop_file_change();
                }),
                Some(move || {
                    // Fail
                    let undo_icon_path = undo_icon_path_fail.clone();
                    self_clone_fail
                        .desktop_file
                        .borrow_mut()
                        .set_icon_path(Path::new(&undo_icon_path));

                    self_clone_fail.on_desktop_file_change();
                    self_clone_fail.on_error("Failed to save icon", None);
                }),
            );
        });
    }

    fn connect_run_app_button(self: &Rc<Self>) {
        let self_clone = self.clone();

        self.run_app_button.connect_clicked(move |_| {
            let desktop_file_borrow = self_clone.desktop_file.borrow();
            let Some(mut executable) = desktop_file_borrow.get_exec() else {
                return;
            };

            if utils::env::is_devcontainer() {
                if desktop_file_borrow
                    .get_browser()
                    .is_some_and(|browser| browser.base == Base::Chromium)
                {
                    let _ = write!(executable, " --no-sandbox");
                }
                debug!("Running in dev-container");
            }

            debug!("Running web app: '{executable}'");
            if let Err(error) = utils::command::run_command_async(&executable) {
                error!(
                    executable = executable,
                    error = error.to_string(),
                    "Failed to run app"
                );
            }
        });
    }

    fn connect_save_button(self: &Rc<Self>) {
        let self_clone = self.clone();

        self.save_button.connect_clicked(move |_| {
            self_clone.on_new_desktop_file_save();
        });
    }

    fn connect_delete_button(self: &Rc<Self>) {
        let self_clone = self.clone();

        self.delete_button.connect_clicked(move |_| {
            debug!(
                "Deleting web app: {}",
                self_clone
                    .desktop_file
                    .borrow()
                    .get_name()
                    .unwrap_or_default()
            );

            if !*self_clone.is_new.borrow() && self_clone.desktop_file.borrow().delete().is_err() {
                self_clone.on_error("Failed to delete all files", None);
            }

            self_clone.nav_view.pop();
        });
    }

    fn connect_name_row(self: &Rc<Self>) {
        let validate_icon = Self::build_validate_icon();
        self.name_row.add_suffix(&validate_icon);

        let self_clone = self.clone();

        self.name_row.connect_changed(move |entry_row| {
            let is_valid = !entry_row.text().is_empty();

            debug!(is_valid = is_valid, "Validate input: {}", entry_row.title());

            validate_icon.set_visible(!is_valid);
            if is_valid {
                entry_row.set_show_apply_button(true);
                entry_row.set_tooltip_text(None);
            } else {
                entry_row.set_show_apply_button(false);
                entry_row.set_tooltip_text(Some("Name is empty"));
            }

            self_clone.on_validate();
        });

        let self_clone = self.clone();

        self.name_row.connect_apply(move |entry_row| {
            let title = entry_row.text();
            if title.is_empty() {
                return;
            }

            self_clone
                .desktop_file
                .borrow_mut()
                .set_name(&entry_row.text());

            self_clone.on_desktop_file_change();
            self_clone.nav_page.set_title(&title);
        });
    }

    fn connect_url_row(self: &Rc<Self>) {
        let validate_icon_url = Self::build_validate_icon();
        let spinner = Spinner::new();
        spinner.set_visible(false);

        self.url_row.add_suffix(&validate_icon_url);
        self.url_row.add_suffix(&spinner);

        let self_clone = self.clone();

        self.url_row.connect_changed(move |entry_row| {
            let is_valid = entry_row.text().validate_url();

            debug!(is_valid = is_valid, "Validate input: {}", entry_row.title());

            validate_icon_url.set_visible(!is_valid);
            if is_valid {
                entry_row.set_show_apply_button(true);
                entry_row.set_tooltip_text(None);
                self_clone.change_icon_button.set_sensitive(true);
            } else {
                entry_row.set_show_apply_button(false);
                entry_row
                    .set_tooltip_text(Some("Please enter a valid URL (e.g., https://example.com)"));
                self_clone.change_icon_button.set_sensitive(false);
            }

            self_clone.on_validate();
        });

        let self_clone = self.clone();
        let running_icon_search_id = Rc::new(RefCell::new(0));

        self.url_row.connect_apply(move |entry_row| {
            self_clone.change_icon_button.set_sensitive(true);

            self_clone
                .desktop_file
                .borrow_mut()
                .set_url(&entry_row.text());

            self_clone
                .desktop_file
                .borrow_mut()
                .set_icon_path(Path::new(""));

            self_clone.reset_app_header();
            self_clone.reset_icon_picker();

            let self_clone = self_clone.clone();
            let spinner_clone = spinner.clone();
            let running_icon_search_id_clone = running_icon_search_id.clone();

            // Bug with clippy, make sure its dropped before await future
            #[allow(clippy::await_holding_refcell_ref)]
            glib::spawn_future_local(async move {
                let mut is_running_icon_search_borrow = running_icon_search_id_clone.borrow_mut();
                *is_running_icon_search_borrow += 1;
                let run_id = *is_running_icon_search_borrow;
                drop(is_running_icon_search_borrow);

                spinner_clone.set_visible(true);
                self_clone.change_icon_button.set_sensitive(false);

                let icon_picker = self_clone.get_icon_picker();
                if let Err(error) = icon_picker.save_first_icon_found().await {
                    if *running_icon_search_id_clone.borrow() != run_id {
                        return;
                    }

                    self_clone
                        .desktop_file
                        .borrow_mut()
                        .set_icon_path(Path::new(""));
                    error!("{error:?}");
                }

                if *running_icon_search_id_clone.borrow() != run_id {
                    return;
                }
                self_clone.on_desktop_file_change();
                spinner_clone.set_visible(false);
                self_clone.change_icon_button.set_sensitive(true);
            });
        });
    }

    fn connect_isolate_row(self: &Rc<Self>) {
        let self_clone = self.clone();

        self.isolate_row.connect_active_notify(move |switch_row| {
            self_clone
                .desktop_file
                .borrow_mut()
                .set_isolated(switch_row.is_active());

            self_clone.on_isolation_change();
            self_clone.on_desktop_file_change();
        });
    }

    fn connect_maximize_row(self: &Rc<Self>) {
        let self_clone = self.clone();

        self.maximize_row.connect_active_notify(move |switch_row| {
            self_clone
                .desktop_file
                .borrow_mut()
                .set_maximized(switch_row.is_active());

            self_clone.on_desktop_file_change();
        });
    }

    fn connect_browser_row(self: &Rc<Self>) {
        let desktop_file_clone = self.desktop_file.clone();
        let self_clone = self.clone();

        self.browser_row
            .connect_selected_item_notify(move |combo_row| {
                let selected_item = combo_row.selected_item();
                let Some(selected_item) = selected_item else {
                    return;
                };
                let browser_item_boxed = selected_item.downcast::<BoxedAnyObject>().unwrap();
                let browser = browser_item_boxed.borrow::<Rc<Browser>>();

                desktop_file_clone.borrow_mut().set_browser(&browser);

                self_clone.on_isolation_change();
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
        let browser_can_isolate = self
            .desktop_file
            .borrow()
            .get_browser()
            .is_some_and(|browser| browser.can_isolate);
        self.isolate_row.set_sensitive(browser_can_isolate);
        if !browser_can_isolate {
            self.isolate_row.set_active(false);
        }
    }

    fn reset_change_icon_button(self: &Rc<Self>) {
        if self
            .desktop_file
            .borrow()
            .get_icon_path()
            .is_some_and(|icon_path| !icon_path.to_string_lossy().is_empty())
        {
            self.change_icon_button.remove_css_class("error");
        } else {
            self.change_icon_button.add_css_class("error");
        }
    }

    fn is_dirty(self: &Rc<Self>) -> bool {
        let desktop_file_borrow = self.desktop_file.borrow();
        let name_saved = desktop_file_borrow.get_name().unwrap_or_default();
        let url_saved = desktop_file_borrow.get_url().unwrap_or_default();
        drop(desktop_file_borrow);

        let is_dirty = name_saved != self.name_row.text() || url_saved != self.url_row.text();

        debug!(is_dirty = is_dirty, "Desktop file dirty validation");

        is_dirty
    }

    fn is_valid(self: &Rc<Self>) -> bool {
        let is_valid = !self.is_dirty() && self.desktop_file.borrow().validate().is_ok();
        debug!(is_valid = is_valid, "Desktop file Validation");

        is_valid
    }

    fn on_validate(self: &Rc<Self>) {
        if *self.is_new.borrow() && self.is_valid() {
            self.save_button.set_sensitive(true);
        } else {
            self.save_button.set_sensitive(false);
        }
    }

    fn on_desktop_file_change(self: &Rc<Self>) {
        debug!("Desktop file changed");

        let is_new = *self.is_new.borrow();

        if is_new {
            self.on_validate();
        }

        if !is_new && let Err(error) = self.desktop_file.borrow_mut().save() {
            match error {
                DesktopFileError::ValidationError(error) => {
                    self.on_error(
                        &format!("Failed to save: '{}'", &error.to_string()),
                        Some(&error.clone().into()),
                    );
                }
                DesktopFileError::Other(error) => {
                    self.on_error("Error saving document", Some(&error));
                }
            }
        }

        self.reset_change_icon_button();
        self.reset_reset_button();
        self.reset_browser_isolation();
        self.reset_app_header();
    }

    fn on_new_desktop_file_save(self: &Rc<Self>) {
        if let Err(error) = self.desktop_file.borrow().validate() {
            match error {
                DesktopFileError::ValidationError(error) => {
                    self.on_error("Invalid input", Some(&error.into()));
                }
                DesktopFileError::Other(error) => {
                    self.on_error("Error saving document", Some(&error));
                }
            }
            return;
        }
        *self.is_new.borrow_mut() = false;
        self.run_app_button.set_visible(true);
        self.save_button.set_visible(false);
        self.on_desktop_file_change();
    }

    fn on_isolation_change(self: &Rc<Self>) {
        let mut desktop_file_borrow = self.desktop_file.borrow_mut();
        let is_isolated = self.isolate_row.is_active();

        let old_profile_path = desktop_file_borrow.get_profile_path().unwrap_or_default();

        let new_profile_path = if is_isolated {
            match desktop_file_borrow.build_profile_path() {
                Err(error) => {
                    drop(desktop_file_borrow);
                    self.reset_desktop_file();
                    self.on_error("Could not set isolation", Some(&error));
                    return;
                }
                Ok(profile) => profile,
            }
        } else {
            PathBuf::default()
        };

        if old_profile_path != new_profile_path && Path::new(&old_profile_path).is_dir() {
            debug!(
                path = old_profile_path.display().to_string(),
                "Deleting profile"
            );
            let _ = fs::remove_dir_all(old_profile_path);
        }

        desktop_file_borrow.set_profile_path(&new_profile_path);
    }

    fn on_error(self: &Rc<Self>, message: &str, error: Option<&anyhow::Error>) {
        if let Some(error) = error {
            error!("{error:?}");
        }
        let toast = Toast::new(message);
        toast.set_timeout(Self::TOAST_MESSAGE_TIMEOUT);
        toast.set_priority(ToastPriority::High);
        self.toast_overlay.dismiss_all();
        self.toast_overlay.add_toast(toast);
    }
}
