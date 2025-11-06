use crate::{application::App, config};
use anyhow::{Context, Result, bail};
use freedesktop_desktop_entry::DesktopEntry;
use gtk::{
    self, Align, Button, ContentFit, FlowBox, Label, Orientation, Picture, SelectionMode,
    gdk_pixbuf::Pixbuf,
    prelude::{BoxExt, ButtonExt, ListBoxRowExt, WidgetExt},
};
use libadwaita::{
    AlertDialog, ButtonContent, ButtonRow, PreferencesGroup, PreferencesPage, PreferencesRow,
    ResponseAppearance, Spinner, StatusPage, Toast, ToastOverlay,
    gio::{Cancellable, MemoryInputStream},
    glib,
    prelude::{AdwDialogExt, AlertDialogExt, PreferencesGroupExt, PreferencesPageExt},
};
use log::{debug, error, info};
use sanitize_filename::sanitize;
use scraper::{Html, Selector};
use std::{cell::RefCell, cmp::Reverse, collections::HashMap, path::Path, rc::Rc};

struct Icon {
    filename: String,
    pixbuf: Pixbuf,
}

pub struct IconPicker {
    prefs_page: PreferencesPage,
    desktop_file: Rc<RefCell<DesktopEntry>>,
    toast_overlay: RefCell<Option<ToastOverlay>>,
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
    const ICON_DIR: &str = "icons";

    pub fn new(desktop_file: &Rc<RefCell<DesktopEntry>>) -> Rc<Self> {
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
            prefs_page,
            desktop_file: desktop_file.clone(),
            toast_overlay: RefCell::new(None),
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

    pub fn init(self: &Rc<Self>, app: &Rc<App>, toast_overlay: Option<&ToastOverlay>) {
        self.load_icons(app);

        let self_clone = self.clone();
        let app_clone = app.clone();
        if let Some(toast_overlay) = toast_overlay {
            *self.toast_overlay.borrow_mut() = Some(toast_overlay.clone());
        }

        self.pref_group_icons_reset_button
            .connect_clicked(move |_| {
                self_clone.load_icons(&app_clone);
            });

        self.pref_group_icons_add_button_row.connect_activated(|_| {
            debug!("TODO");
        });
    }

    pub fn show_dialog(self: &Rc<Self>, app: &Rc<App>) {
        let dialog = AlertDialog::builder()
            .heading("Pick an icon")
            .width_request(500)
            .extra_child(&self.content_box)
            .build();
        dialog.add_response("cancel", "_Cancel");
        dialog.add_response("save", "_Save");
        dialog.set_response_appearance("save", ResponseAppearance::Suggested);
        dialog.set_default_response(Some("save"));
        dialog.set_close_response("cancel");

        let self_clone = self.clone();
        let app_clone = app.clone();
        dialog.connect_response(Some("save"), move |_, _| {
            let icon = match self_clone.get_selected_icon() {
                Ok(icon) => icon,
                Err(error) => {
                    error!("{error:?}");
                    return;
                }
            };

            if let Err(error) = self_clone.save_icon(&app_clone, &icon) {
                error!("{error:?}");
            }
        });

        dialog.present(Some(&app.window.adw_window));
    }

    fn save_icon(&self, app: &Rc<App>, icon: &Rc<Icon>) -> Result<()> {
        let mut desktop_file = self.desktop_file.borrow_mut();
        let app_id = desktop_file
            .desktop_entry(config::DesktopFile::ID_KEY)
            .context("No app id on desktop file!")?;
        let data_dir = app
            .dirs
            .get_data_home()
            .context("No data dir???")?
            .to_string_lossy()
            .to_string();

        let filename = match Path::new(&icon.filename).extension() {
            Some(extension) => {
                if extension == "png" {
                    icon.filename.clone()
                } else {
                    format!("{}.png", icon.filename)
                }
            }
            None => format!("{}.png", icon.filename),
        };

        let save_dir = format!("{data_dir}{}", Self::ICON_DIR);
        let icon_name = sanitize(format!("{app_id}-{filename}"));
        let save_path = format!("{save_dir}/{icon_name}");

        let toast_overlay = self.toast_overlay.borrow().clone();

        debug!("Saving {icon_name} to fs: {save_path}");
        let save_to_fs = || -> Result<()> {
            app.dirs
                .place_data_file(&save_path)
                .context("Failed to create paths")?;
            icon.pixbuf
                .savev(save_path.clone(), "png", &[])
                .context("Failed to save to fs")?;
            Ok(())
        };

        if let Err(error) = save_to_fs() {
            if let Some(toast_overlay) = toast_overlay {
                toast_overlay.add_toast(Toast::new("Error saving icon"));
            }
            bail!(error)
        }

        if let Some(toast_overlay) = toast_overlay {
            toast_overlay.add_toast(Toast::new("Saved icon"));
        }
        desktop_file.add_desktop_entry("Icon".to_string(), save_path);

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

    fn load_icons(self: &Rc<Self>, app: &Rc<App>) {
        let self_clone = self.clone();
        let app_clone = app.clone();
        let url: String;
        {
            let desktop_file_borrow = self_clone.desktop_file.borrow();
            url = desktop_file_borrow
                .desktop_entry(config::DesktopFile::URL_KEY)
                .unwrap_or_default()
                .to_string();
        }

        glib::spawn_future_local(async move {
            self_clone.prefs_page.set_visible(false);
            self_clone.spinner.set_visible(true);
            self_clone.pref_row_icons.set_visible(false);
            self_clone.pref_row_icons_fail.set_visible(true);

            if let Err(error) = self_clone.set_online_icons(&url, &app_clone).await {
                error!("{error:?}");
                self_clone.pref_row_icons.set_visible(false);
                self_clone.pref_row_icons_fail.set_visible(true);
            } else {
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
                *self_clone.pref_row_icons_flow_box.borrow_mut() = Some(flow_box);

                self_clone.pref_row_icons.set_visible(true);
                self_clone.pref_row_icons_fail.set_visible(false);
            }

            self_clone.spinner.set_visible(false);
            self_clone.prefs_page.set_visible(true);
        });
    }

    async fn set_online_icons(&self, url: &str, app: &Rc<App>) -> Result<()> {
        debug!("Fetching online icons");
        let url_clone = url.to_string();

        let html_text = app.fetch.get_as_string(url_clone).await?;
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
            let app_clone = app.clone();
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
            let Some(filename) = url.split('/').next_back() else {
                error!("Failed to parse url for filename: '{url}'");
                continue;
            };
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
            let icon = Icon {
                filename: filename.to_string(),
                pixbuf,
            };
            self.icons.borrow_mut().insert(url, Rc::new(icon));
        }

        if self.icons.borrow().is_empty() {
            bail!("No icons found for: {url}")
        }

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
