use crate::application::App;
use anyhow::{Context, Result, bail};
use common::desktop_file::{DesktopFile, Icon};
use gtk::{
    self, Align, Button, ContentFit, FileDialog, FileFilter, FlowBox, FlowBoxChild, Label,
    Orientation, Picture, SelectionMode,
    gdk_pixbuf::Pixbuf,
    gio::prelude::FileExt,
    glib::object::Cast,
    prelude::{BoxExt, ButtonExt, ListBoxRowExt, WidgetExt},
};
use libadwaita::{
    AlertDialog, ButtonContent, ButtonRow, PreferencesGroup, PreferencesPage, PreferencesRow,
    ResponseAppearance, Spinner, StatusPage,
    gio::{Cancellable, MemoryInputStream},
    glib,
    prelude::{AdwDialogExt, AlertDialogExt, PreferencesGroupExt, PreferencesPageExt},
};
use scraper::{Html, Selector};
use std::{cell::RefCell, cmp::Reverse, collections::HashMap, fs, rc::Rc};
use tracing::{debug, error, info};

pub struct IconPicker {
    init: RefCell<bool>,
    prefs_page: PreferencesPage,
    app: Rc<App>,
    desktop_file: Rc<RefCell<DesktopFile>>,
    icons: Rc<RefCell<HashMap<String, Rc<Icon>>>>,
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

    pub fn new(app: &Rc<App>, desktop_file: &Rc<RefCell<DesktopFile>>) -> Rc<Self> {
        let icons = Rc::new(RefCell::new(HashMap::new()));
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

        Rc::new(Self {
            init: RefCell::new(false),
            prefs_page,
            app: app.clone(),
            desktop_file: desktop_file.clone(),
            icons,
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
        if *is_init {
            return;
        }
        self.load_icons();

        let self_clone = self.clone();
        self.pref_group_icons_reset_button
            .connect_clicked(move |_| {
                self_clone.load_icons();
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

    pub async fn set_first_icon(self: &Rc<Self>, url: &str) -> Result<()> {
        self.set_online_icons(url).await?;
        let icons_borrow = self.icons.borrow();
        let mut icons: Vec<(&String, &Rc<Icon>)> = icons_borrow.iter().collect();
        icons.sort_by_key(|(_, a)| Reverse(a.pixbuf.byte_length()));

        let Some((_url, icon)) = icons.first() else {
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

    fn load_icons(self: &Rc<Self>) {
        let self_clone = self.clone();
        let url = self_clone
            .desktop_file
            .borrow()
            .get_url()
            .unwrap_or_default();

        glib::spawn_future_local(async move {
            self_clone.prefs_page.set_visible(false);
            self_clone.spinner.set_visible(true);
            self_clone.pref_row_icons.set_visible(false);
            self_clone.pref_row_icons_fail.set_visible(true);

            if let Err(error) = self_clone.set_online_icons(&url).await {
                error!("{error:?}");
                self_clone.prefs_page.set_visible(true);
                self_clone.spinner.set_visible(false);
                self_clone.pref_row_icons.set_visible(false);
                self_clone.pref_row_icons_fail.set_visible(true);
                return;
            }

            self_clone.reload_icons();
            self_clone.prefs_page.set_visible(true);
            self_clone.spinner.set_visible(false);
            self_clone.pref_row_icons.set_visible(true);
            self_clone.pref_row_icons_fail.set_visible(false);
        });
    }

    fn reload_icons(self: &Rc<Self>) {
        let self_clone = self.clone();
        let flow_box = Self::build_pref_row_icons_flow_box();
        let pref_row_icons = &self_clone.pref_row_icons;
        pref_row_icons.set_child(Some(&flow_box));

        let icons = self_clone.icons.borrow();
        let mut icons: Vec<(&String, &Rc<Icon>)> = icons.iter().collect();

        icons.sort_by_key(|(_, a)| Reverse(a.pixbuf.byte_length()));

        for (url, icon) in icons {
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

        *self_clone.pref_row_icons_flow_box.borrow_mut() = Some(flow_box);
    }

    async fn set_online_icons(&self, url: &str) -> Result<()> {
        debug!("Fetching online icons");
        let url_clone = url.to_string();

        let html_text = self.app.fetch.get_as_string(url_clone).await?;
        let fragment = Html::parse_document(&html_text);
        let selector =
            Selector::parse("link[rel~=\"icon\"], link[rel~=\"shortcut\"][rel~=\"icon\"]").unwrap();

        let mut urls = Vec::new();
        for element in fragment.select(&selector) {
            if let Some(href) = element.value().attr("href") {
                info!("Favicon found: {href}");
                urls.push(href.to_string());
            }
        }

        let mut handles = Vec::new();
        for url in urls {
            let app_clone = self.app.clone();
            let url_clone = url.clone();
            // Spawn in parallel on main thread
            let handle =
                glib::spawn_future_local(
                    async move { app_clone.fetch.get_as_bytes(url_clone).await },
                );
            handles.push((handle, url));
        }

        for handle in handles {
            let (handle, url) = handle;
            let Ok(Ok(image_bytes)) = handle.await else {
                error!("Failed to fetch image: '{url}'");
                continue;
            };
            let g_bytes = glib::Bytes::from(&image_bytes);
            let stream = MemoryInputStream::from_bytes(&g_bytes);
            let Ok(pixbuf) = Pixbuf::from_stream(&stream, Cancellable::NONE) else {
                error!("Failed to convert image: '{url}'");
                continue;
            };
            let icon = Icon { pixbuf };
            self.icons.borrow_mut().insert(url, Rc::new(icon));
        }

        if self.icons.borrow().is_empty() {
            bail!("No icons found for: {url}")
        }

        Ok(())
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

                let icon = Icon { pixbuf };
                self_clone
                    .icons
                    .borrow_mut()
                    .insert(filename, Rc::new(icon));

                self_clone.reload_icons();
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
            .max_children_per_line(5)
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
