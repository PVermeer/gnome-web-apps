use crate::{application::App, config};
use anyhow::Result;
use freedesktop_desktop_entry::DesktopEntry;
use gtk::{
    self, Align, Button, ContentFit, FlowBox, Label, Orientation, Picture, SelectionMode,
    gdk_pixbuf::Pixbuf,
    prelude::{BoxExt, ButtonExt, ListBoxRowExt, WidgetExt},
};
use libadwaita::{
    AlertDialog, ButtonContent, ButtonRow, PreferencesGroup, PreferencesPage, PreferencesRow,
    ResponseAppearance, Spinner, StatusPage,
    gio::{Cancellable, MemoryInputStream},
    glib,
    prelude::{AdwDialogExt, AlertDialogExt, PreferencesGroupExt, PreferencesPageExt},
};
use log::{debug, error, info};
use scraper::{Html, Selector};
use std::{cell::RefCell, cmp::Reverse, collections::HashMap, rc::Rc, time::Duration};

pub struct IconPicker {
    prefs_page: PreferencesPage,
    desktop_file: Rc<RefCell<DesktopEntry>>,
    icons: Rc<RefCell<HashMap<String, Pixbuf>>>,
    pref_row_icons: PreferencesRow,
    pref_row_icons_fail: PreferencesRow,
    pref_group_icons_reset_button: Button,
    pref_group_icons_add_button_row: ButtonRow,
    content_box: gtk::Box,
    spinner: Spinner,
}
impl IconPicker {
    const FETCH_TIMEOUT: u64 = 5; // Seconds

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
            icons,
            pref_row_icons,
            pref_row_icons_fail,
            pref_group_icons_reset_button,
            pref_group_icons_add_button_row,
            content_box,
            spinner,
        })
    }

    pub fn init(self: &Rc<Self>) {
        self.reset();

        let self_clone = self.clone();

        self.pref_group_icons_reset_button
            .connect_clicked(move |_| {
                self_clone.reset();
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

        dialog.connect_response(Some("save"), |_, _| {
            debug!("TODO Saving icon");
        });

        dialog.present(Some(&app.window.adw_window));
    }

    fn reset(self: &Rc<Self>) {
        let self_clone = self.clone();
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

            if let Err(error) = self_clone.set_online_icons(&url).await {
                error!("{error}");
                self_clone.pref_row_icons.set_visible(false);
                self_clone.pref_row_icons_fail.set_visible(true);
            } else {
                let flow_box = Self::build_pref_row_icons_flow_box();
                let pref_row_icons = &self_clone.pref_row_icons;
                pref_row_icons.set_child(Some(&flow_box));

                let icons = self_clone.icons.borrow();
                let mut icons: Vec<(&String, &Pixbuf)> = icons.iter().collect();

                icons.sort_by_key(|(_, a)| Reverse(a.byte_length()));

                for (_, icon) in icons {
                    let frame = gtk::Box::new(Orientation::Vertical, 0);
                    let picture = Picture::new();
                    picture.set_pixbuf(Some(icon));
                    picture.set_content_fit(ContentFit::ScaleDown);
                    frame.append(&picture);

                    let size_text = format!("{} x {}", icon.width(), icon.height());
                    let label = Label::builder().label(&size_text).build();
                    frame.append(&label);

                    flow_box.insert(&frame, -1);
                }

                self_clone.pref_row_icons.set_visible(true);
                self_clone.pref_row_icons_fail.set_visible(false);
            }

            self_clone.spinner.set_visible(false);
            self_clone.prefs_page.set_visible(true);
        });
    }

    async fn set_online_icons(self: &Rc<Self>, url: &str) -> Result<()> {
        debug!("Fetching online icons");
        let url = url.to_string();
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(Self::FETCH_TIMEOUT))
            .connect_timeout(Duration::from_secs(Self::FETCH_TIMEOUT))
            .build()?;

        let html_text = tokio::spawn(client.get(url).send()).await??.text().await?;
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
            let client = client.clone();
            let handle = tokio::spawn(async move {
                debug!("Fetching icon: {url}");
                let response = client.get(&url).send().await?;
                let bytes = response.bytes().await?;
                Ok::<_, reqwest::Error>((bytes, url))
            });
            handles.push(handle);
        }

        let mut results = Vec::new();
        for handle in handles {
            let result = handle.await??;
            results.push(result);
        }

        let mut icons = self.icons.borrow_mut();
        for (image_bytes, url) in results {
            debug!("Converting image data to Pixbuf: {url}");
            let g_bytes = glib::Bytes::from(&image_bytes);
            let stream = MemoryInputStream::from_bytes(&g_bytes);
            let Ok(pixbuf) = Pixbuf::from_stream(&stream, Cancellable::NONE) else {
                error!("Failed to convert image: {url}");
                continue;
            };

            icons.insert(url, pixbuf);
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
