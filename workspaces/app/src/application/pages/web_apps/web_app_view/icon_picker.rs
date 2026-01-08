mod icon_fetcher;

use crate::application::App;
use anyhow::{Context, Result, bail};
use common::desktop_file::{DesktopFile, Icon};
use gtk::{
    self, Align, Button, ContentFit, FileDialog, FileFilter, FlowBox, FlowBoxChild, Label,
    Orientation, Picture, SelectionMode,
    gdk_pixbuf::Pixbuf,
    gio::prelude::FileExt,
    glib::object::Cast,
    prelude::{BoxExt, ButtonExt, FlowBoxChildExt, ListBoxRowExt, WidgetExt},
};
use icon_fetcher::IconFetcher;
use libadwaita::{
    AlertDialog, ButtonContent, ButtonRow, PreferencesGroup, PreferencesPage, PreferencesRow,
    ResponseAppearance, Spinner, StatusPage,
    gio::Cancellable,
    glib,
    prelude::{AdwDialogExt, AlertDialogExt, PreferencesGroupExt, PreferencesPageExt},
};
use std::{
    cell::RefCell,
    cmp::Reverse,
    collections::HashMap,
    fs,
    rc::Rc,
    time::{Duration, SystemTime},
};
use tracing::{debug, error};

pub struct IconPicker {
    init: RefCell<bool>,
    fetched_icons_ts: RefCell<SystemTime>,
    prefs_page: PreferencesPage,
    app: Rc<App>,
    desktop_file: Rc<RefCell<DesktopFile>>,
    icons: Rc<RefCell<HashMap<String, Rc<Icon>>>>,
    icons_ordered: RefCell<Vec<(String, Rc<Icon>)>>,
    pref_row_icons: PreferencesRow,
    pref_row_icons_fail: PreferencesRow,
    pref_row_icons_flow_box: RefCell<Option<FlowBox>>,
    pref_group_icons_reset_button: Button,
    pref_group_icons_add_button_row: ButtonRow,
    content_box: gtk::Box,
    spinner: Spinner,
}
impl IconPicker {
    pub const DIALOG_SAVE: &str = "save";
    pub const DIALOG_CANCEL: &str = "cancel";
    /// In seconds
    pub const ONLINE_FETCH_THROTTLE: u64 = 20;

    pub fn new(app: &Rc<App>, desktop_file: &Rc<RefCell<DesktopFile>>) -> Rc<Self> {
        let icons = Rc::new(RefCell::new(HashMap::new()));
        let icons_ordered = RefCell::new(Vec::new());
        let content_box = gtk::Box::new(Orientation::Horizontal, 0);
        let spinner = Self::build_spinner();
        let prefs_page = PreferencesPage::new();
        let pref_row_icons = Self::build_pref_row_icons();
        let pref_row_icons_fail = Self::build_pref_row_icons_fail();
        let (pref_group_icons, pref_group_icons_reset_button) = Self::build_pref_group_icons();
        let pref_group_icons_add_button_row = Self::build_pref_row_add_icon();

        prefs_page.add(&pref_group_icons);
        pref_group_icons.add(&pref_row_icons);
        pref_group_icons.add(&pref_row_icons_fail);
        pref_group_icons.add(&pref_group_icons_add_button_row);

        content_box.append(&spinner);
        content_box.append(&prefs_page);

        let fetched_icons_ts = RefCell::new(
            SystemTime::now()
                .checked_sub(Duration::from_secs(Self::ONLINE_FETCH_THROTTLE + 5))
                .unwrap_or(SystemTime::now()),
        );

        Rc::new(Self {
            init: RefCell::new(false),
            fetched_icons_ts,
            prefs_page,
            app: app.clone(),
            desktop_file: desktop_file.clone(),
            icons,
            icons_ordered,
            pref_row_icons,
            pref_row_icons_fail,
            pref_row_icons_flow_box: RefCell::new(None),
            pref_group_icons_reset_button,
            pref_group_icons_add_button_row,
            content_box,
            spinner,
        })
    }

    pub fn init(self: &Rc<Self>) {
        let mut is_init = self.init.borrow_mut();
        self.load_icons(false);

        if *is_init {
            return;
        }

        let self_clone = self.clone();
        self.pref_group_icons_reset_button
            .connect_clicked(move |_| {
                self_clone.load_icons(true);
            });

        let self_clone = self.clone();
        self.pref_group_icons_add_button_row
            .connect_activated(move |_| {
                self_clone.load_icon_file_picker();
            });

        *is_init = true;
    }

    pub fn show_dialog<Success, Fail>(
        self: &Rc<Self>,
        success_cb: Option<Success>,
        fail_cb: Option<Fail>,
    ) -> AlertDialog
    where
        Success: Fn() + 'static,
        Fail: Fn() + 'static,
    {
        self.init();

        let dialog = AlertDialog::builder()
            .heading("Pick an icon")
            .width_request(500)
            .extra_child(&self.content_box)
            .build();
        dialog.add_response(Self::DIALOG_CANCEL, "_Cancel");
        dialog.add_response(Self::DIALOG_SAVE, "_Save");
        dialog.set_response_appearance(Self::DIALOG_SAVE, ResponseAppearance::Suggested);
        dialog.set_default_response(Some(Self::DIALOG_CANCEL));
        dialog.set_close_response(Self::DIALOG_CANCEL);

        let self_clone = self.clone();
        dialog.connect_response(
            Some(Self::DIALOG_SAVE),
            move |_, _| match (|| -> Result<()> {
                let icon = self_clone.get_selected_icon()?;
                self_clone.save(&icon)?;
                Ok(())
            })() {
                Ok(()) => {
                    if let Some(success_cb) = &success_cb {
                        success_cb();
                    }
                }
                Err(error) => {
                    error!("Error saving icon: {error:?}");
                    if let Some(fail_cb) = &fail_cb {
                        fail_cb();
                    }
                }
            },
        );

        dialog.present(Some(&self.app.window.adw_window));
        dialog
    }

    pub async fn save_first_icon_found(self: &Rc<Self>) -> Result<()> {
        self.set_online_icons(false).await?;
        let icons_ordered_borrow = self.icons_ordered.borrow();

        let Some((_url, icon)) = icons_ordered_borrow.first() else {
            bail!("No icons found")
        };

        self.save(icon)?;
        Ok(())
    }

    fn get_selected_icon(self: &Rc<Self>) -> Result<Rc<Icon>> {
        let url_or_path = self
            .clone()
            .pref_row_icons_flow_box
            .borrow()
            .clone()
            .context("Flow box does not exist")?
            .selected_children()
            .first()
            .context("Flowbox does not have a selected item")?
            .first_child()
            .context("Could not get container of selected flowbox item")?
            .widget_name()
            .to_string();

        let icon = self
            .icons
            .borrow()
            .get(&url_or_path)
            .context("Cannot find icon in HashMap???")?
            .clone();
        Ok(icon)
    }

    fn set_icons_loading(&self) {
        self.prefs_page.set_visible(false);
        self.spinner.set_visible(true);
        self.pref_row_icons.set_visible(false);
        self.pref_row_icons_fail.set_visible(true);
    }

    fn set_no_icons(&self) {
        self.prefs_page.set_visible(true);
        self.spinner.set_visible(false);
        self.pref_row_icons.set_visible(false);
        self.pref_row_icons_fail.set_visible(true);
    }

    fn set_show_icons(&self) {
        self.prefs_page.set_visible(true);
        self.spinner.set_visible(false);
        self.pref_row_icons.set_visible(true);
        self.pref_row_icons_fail.set_visible(false);
    }

    fn select_icon(&self, filename: &str) {
        let flow_box_borrow = self.pref_row_icons_flow_box.borrow();
        if let Some(flow_box) = flow_box_borrow.as_ref() {
            let mut idx = 0;
            while let Some(flow_box_child) = flow_box.child_at_index(idx) {
                if let Some(widget) = flow_box_child.child()
                    && filename == widget.widget_name()
                {
                    flow_box.select_child(&flow_box_child);
                    break;
                }

                idx += 1;
            }
        }
    }

    fn load_icons(self: &Rc<Self>, force: bool) {
        let self_clone = self.clone();

        glib::spawn_future_local(async move {
            self_clone.set_icons_loading();

            if let Err(error) = self_clone.set_online_icons(force).await {
                error!("{error:?}");
                self_clone.set_no_icons();
                return;
            }

            self_clone.reload_icons();
        });
    }

    fn reload_icons(self: &Rc<Self>) {
        let self_clone = self.clone();
        let flow_box = Self::build_pref_row_icons_flow_box();
        let pref_row_icons = &self_clone.pref_row_icons;
        pref_row_icons.set_child(Some(&flow_box));

        let icons_ordered_borrow = self_clone.icons_ordered.borrow();

        for (url, icon) in icons_ordered_borrow.iter() {
            let frame = gtk::Box::new(Orientation::Vertical, 0);
            frame.set_widget_name(url);
            let picture = Picture::new();
            picture.set_pixbuf(Some(&icon.pixbuf));
            picture.set_content_fit(ContentFit::ScaleDown);
            frame.append(&picture);

            let size_text = format!("{} x {}", icon.pixbuf.width(), icon.pixbuf.height());
            let label = Label::builder().label(&size_text).build();
            frame.append(&label);

            flow_box.insert(&frame, -1);
        }

        if let Some(first_child) = flow_box.first_child() {
            let flow_box_child = first_child.downcast_ref::<FlowBoxChild>();
            if let Some(flow_box_child) = flow_box_child {
                flow_box.select_child(flow_box_child);
            }
        }

        if icons_ordered_borrow.is_empty() {
            self.set_no_icons();
        } else {
            self.set_show_icons();
        }

        *self_clone.pref_row_icons_flow_box.borrow_mut() = Some(flow_box);
    }

    async fn set_online_icons(self: &Rc<Self>, force: bool) -> Result<()> {
        if !force && self.should_throttle() {
            return Ok(());
        }

        debug!("Fetching online icons");

        let Some(url) = self.desktop_file.borrow().get_url() else {
            bail!("No url on desktop file")
        };
        let Ok(mut icon_fetcher) = IconFetcher::new(&self.app, &url) else {
            bail!("Invalid url")
        };
        let Ok(icons) = icon_fetcher.get_online_icons().await else {
            bail!("Failed to get online icons")
        };

        let mut self_icons_borrow = self.icons.borrow_mut();
        let mut self_icons_ordered_borrow = self.icons_ordered.borrow_mut();

        for (url, icon) in icons {
            self_icons_borrow.insert(url, icon);
        }

        *self_icons_ordered_borrow = self_icons_borrow.clone().into_iter().collect();
        self_icons_ordered_borrow.sort_by_key(|(_, a)| Reverse(a.pixbuf.byte_length()));

        if self_icons_borrow.is_empty() {
            bail!("No icons found for: {url}")
        }

        Ok(())
    }

    fn should_throttle(self: &Rc<Self>) -> bool {
        let now = SystemTime::now();
        let throttle_duration = Duration::from_secs(Self::ONLINE_FETCH_THROTTLE);
        let previous_fetch_ts = *self.fetched_icons_ts.borrow();
        let previous_fetch_duration = now.duration_since(previous_fetch_ts).unwrap();

        if previous_fetch_duration < throttle_duration {
            return true;
        }

        *self.fetched_icons_ts.borrow_mut() = SystemTime::now();

        false
    }

    fn load_icon_file_picker(self: &Rc<Self>) {
        debug!("Opening file picker");

        let file_filter = FileFilter::new();
        file_filter.set_name(Some("Images"));
        file_filter.add_mime_type("image/png");
        file_filter.add_mime_type("image/jpeg");

        let file_dialog = FileDialog::builder()
            .title("Pick an image")
            .default_filter(&file_filter)
            .build();

        let self_clone = self.clone();
        let app_clone = self.app.clone();

        file_dialog.open(
            Some(&app_clone.window.adw_window),
            None::<&Cancellable>,
            move |file| {
                let Ok(file) = file else {
                    error!("Could not get file");
                    return;
                };
                let Some(path) = file.path() else {
                    error!("Could not get path");
                    return;
                };

                let filename = file.parse_name().to_string();
                debug!("Loading image: '{filename}'");

                let pixbuf = match Pixbuf::from_file(&path) {
                    Err(error) => {
                        error!("Could not load image into a Pixbuf: '{error:?}'");
                        return;
                    }
                    Ok(pixbuf) => pixbuf,
                };

                let icon = Rc::new(Icon { pixbuf });
                self_clone
                    .icons
                    .borrow_mut()
                    .insert(filename.clone(), icon.clone());
                self_clone
                    .icons_ordered
                    .borrow_mut()
                    .push((filename.clone(), icon.clone()));

                self_clone.reload_icons();
                self_clone.select_icon(&filename);
            },
        );
    }

    fn save(self: &Rc<Self>, icon: &Rc<Icon>) -> Result<()> {
        let mut desktop_file_borrow = self.desktop_file.borrow_mut();
        if let Some(old_icon_path) = desktop_file_borrow.get_icon_path()
            && old_icon_path.is_file()
        {
            fs::remove_file(old_icon_path).context("Failed to remove old icon")?;
        }

        let app_id = desktop_file_borrow
            .get_id()
            .context("No file id on DesktopFile")?;

        let icon_dir = self.app.dirs.icons();
        let file_name = sanitize_filename::sanitize(format!("{app_id}.png"));
        let save_path = icon_dir.join(&file_name);

        debug!(
            "Saving icon '{}' to fs: {}",
            &file_name,
            save_path.display()
        );

        icon.pixbuf
            .savev(save_path.clone(), "png", &[])
            .context("Failed to save icon to fs")?;

        desktop_file_borrow.set_icon_path(&save_path);
        drop(desktop_file_borrow);

        Ok(())
    }

    fn build_spinner() -> Spinner {
        Spinner::builder()
            .height_request(48)
            .width_request(96)
            .halign(Align::Center)
            .valign(Align::Center)
            .hexpand(true)
            .vexpand(true)
            .build()
    }

    fn build_pref_group_icons() -> (PreferencesGroup, Button) {
        let content = ButtonContent::builder()
            .label("Reset")
            .icon_name("folder-download-symbolic")
            .build();
        let button = Button::builder()
            .css_classes(["flat"])
            .child(&content)
            .build();

        let pref_group = PreferencesGroup::builder()
            .title("Icons")
            .header_suffix(&button)
            .build();

        (pref_group, button)
    }

    fn build_pref_row_add_icon() -> ButtonRow {
        ButtonRow::builder()
            .title("Add icon")
            .start_icon_name("list-add-symbolic")
            .build()
    }

    fn build_pref_row_icons_flow_box() -> FlowBox {
        FlowBox::builder()
            .height_request(96)
            .column_spacing(10)
            .row_spacing(10)
            .homogeneous(false)
            .max_children_per_line(4)
            .min_children_per_line(4)
            .selection_mode(SelectionMode::Single)
            .build()
    }

    fn build_pref_row_icons() -> PreferencesRow {
        PreferencesRow::builder().build()
    }

    fn build_pref_row_icons_fail() -> PreferencesRow {
        let status_page = StatusPage::builder()
            .title("No icons found")
            .description("Try adding one")
            .css_classes(["compact"])
            .build();

        PreferencesRow::builder().child(&status_page).build()
    }
}
