use crate::{application::App, config};
use anyhow::Result;
use freedesktop_desktop_entry::DesktopEntry;
use libadwaita::{
    AlertDialog, PreferencesGroup, PreferencesPage, PreferencesRow, ResponseAppearance, Spinner,
    gio::{Cancellable, MemoryInputStream},
    glib,
    gtk::{
        self, Align, FlowBox, Label, Orientation, Picture, SelectionMode,
        gdk_pixbuf::Pixbuf,
        prelude::{BoxExt, WidgetExt},
    },
    prelude::{AdwDialogExt, AlertDialogExt, PreferencesGroupExt, PreferencesPageExt},
};
use log::{debug, error, info};
use scraper::{Html, Selector};
use std::{cell::RefCell, rc::Rc};

pub struct IconPicker {
    prefs_page: PreferencesPage,
    desktop_file: Rc<RefCell<DesktopEntry>>,
    content_box: gtk::Box,
    flowbox: FlowBox,
    spinner: Spinner,
}
impl IconPicker {
    pub fn new(desktop_file: &Rc<RefCell<DesktopEntry>>) -> Rc<Self> {
        let content_box = gtk::Box::new(Orientation::Horizontal, 0);
        let spinner = Spinner::builder()
            .height_request(48)
            .width_request(48)
            .halign(Align::Center)
            .valign(Align::Center)
            .hexpand(true)
            .vexpand(true)
            .build();
        let flowbox = FlowBox::builder()
            .column_spacing(10)
            .row_spacing(10)
            .homogeneous(true)
            .max_children_per_line(5)
            .selection_mode(SelectionMode::Single)
            .build();
        content_box.append(&spinner);

        let prefs_page = PreferencesPage::new();
        let pref_group = PreferencesGroup::builder().title("Online icons").build();
        let pref_row = PreferencesRow::builder().child(&flowbox).build();
        pref_group.add(&pref_row);
        prefs_page.add(&pref_group);

        Rc::new(Self {
            prefs_page,
            desktop_file: desktop_file.clone(),
            content_box,
            flowbox,
            spinner,
        })
    }

    pub fn init(self: &Rc<Self>) {
        let url: String;
        {
            let desktop_file_borrow = self.desktop_file.borrow();
            url = desktop_file_borrow
                .desktop_entry(config::DesktopFile::URL_KEY)
                .unwrap_or_default()
                .to_string();
        }

        let self_clone = self.clone();
        glib::spawn_future_local(async move {
            let online_icons = self_clone.get_online_icons(&url).await.unwrap();

            for icon in online_icons {
                let frame = gtk::Box::new(Orientation::Vertical, 0);
                frame.set_size_request(96, 96);
                let picture = Picture::new();
                picture.set_pixbuf(Some(&icon));
                frame.append(&picture);

                let label = Label::builder()
                    .label(format!("{} x {}", icon.width(), icon.height()))
                    .build();
                frame.append(&label);

                self_clone.flowbox.insert(&frame, -1);
            }
            self_clone.content_box.remove(&self_clone.spinner);
            self_clone.content_box.append(&self_clone.prefs_page);
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

    async fn get_online_icons(self: &Rc<Self>, url: &str) -> Result<Vec<Pixbuf>> {
        let url = url.to_string();
        let mut icons = Vec::new();
        let client = reqwest::Client::new();

        let html_text = reqwest::get(url).await.unwrap().text().await.unwrap();
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
                let response = client.get(&url).send().await?;
                let bytes = response.bytes().await?;
                Ok::<_, reqwest::Error>((bytes, url))
            });
            handles.push(handle);
        }

        for handle in handles {
            let (image_bytes, url) = handle.await.unwrap().unwrap();
            let g_bytes = glib::Bytes::from(&image_bytes);
            let stream = MemoryInputStream::from_bytes(&g_bytes);
            let Ok(pixbuf) = Pixbuf::from_stream(&stream, Cancellable::NONE) else {
                error!("Failed to convert image: {url}");
                continue;
            };

            icons.push(pixbuf);
        }

        Ok(icons)
    }
}
