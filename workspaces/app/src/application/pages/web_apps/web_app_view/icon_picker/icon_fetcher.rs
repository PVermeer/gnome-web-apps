use crate::application::App;
use anyhow::{Result, bail};
use common::{desktop_file::Icon, url::UrlExt};
use gtk::{
    gdk_pixbuf::Pixbuf,
    gio::{Cancellable, MemoryInputStream},
    glib::{self},
};
use scraper::{Html, Selector};
use serde::Deserialize;
use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};
use tracing::{debug, error, info};
use url::Url;

#[derive(Deserialize)]
struct ManifestIcon {
    src: Option<String>,
}
#[derive(Deserialize)]
struct ManifestJson {
    icons: Option<Vec<ManifestIcon>>,
}

pub struct IconFetcher {
    app: Rc<App>,
    url: Url,
    base_url: Option<Url>,
    icon_urls: HashSet<String>,
    /// `HashMap<manifest_url_string, (manifest_url, base_url)>`
    manifest_urls: HashMap<String, (Url, Url)>,
}
impl IconFetcher {
    pub fn new(app: &Rc<App>, url: &str) -> Result<Self> {
        let Some(url) = Url::parse(url).ok() else {
            bail!("Invalid url")
        };
        let base_url = if url.has_path() {
            url.get_base_url().ok()
        } else {
            None
        };

        Ok(Self {
            app: app.clone(),
            url,
            base_url,
            icon_urls: HashSet::new(),
            manifest_urls: HashMap::new(),
        })
    }

    pub async fn get_online_icons(&mut self) -> Result<Vec<(String, Rc<Icon>)>> {
        debug!("Fetching online icons");

        let urls = [Some(self.url.clone()), self.base_url.clone()];

        for url in urls {
            let Some(url) = url else {
                continue;
            };
            let html_text = self.app.fetch.get_as_string(url.as_str()).await?;
            let fragment = Html::parse_document(&html_text);

            self.set_default_icon_urls(&url);
            self.set_manifest_urls_from_html(&fragment, &url);
            self.set_icon_urls_from_html(&fragment, &url);
        }
        self.set_icon_urls_from_manifests().await;

        let icons = self.fetch_icons_from_urls().await;

        Ok(icons)
    }

    #[allow(clippy::unused_self)]
    fn get_href_as_absolute_url(&self, href: &str, url: &Url) -> Result<Url> {
        let sanitized_url = url.sanitize();
        let new_url = Url::parse(href).or(sanitized_url.join(href))?;
        Ok(new_url)
    }

    fn set_default_icon_urls(&mut self, url: &Url) {
        let sanitized_url = url.sanitize();
        let default_urls = [sanitized_url.join("favicon.ico").ok()];

        for default_url in default_urls {
            let Some(default) = default_url else {
                continue;
            };
            self.icon_urls.insert(default.to_string());
        }
    }

    fn set_manifest_urls_from_html(&mut self, html_fragment: &Html, url: &Url) {
        let Ok(manifest_selector) = Selector::parse("link[rel~=\"manifest\"]") else {
            return;
        };

        for element in html_fragment.select(&manifest_selector) {
            if let Some(href) = element.value().attr("href") {
                debug!(href, "Manifest found");
                let Ok(manifest_url) = self.get_href_as_absolute_url(href, url) else {
                    continue;
                };
                info!(url = manifest_url.to_string(), "Manifest url found");
                self.manifest_urls
                    .insert(manifest_url.to_string(), (manifest_url, url.clone()));
            }
        }
    }

    fn set_icon_urls_from_html(&mut self, html_fragment: &Html, url: &Url) {
        let Ok(icon_selector) =
            Selector::parse("link[rel~=\"icon\"], link[rel~=\"shortcut\"][rel~=\"icon\"]")
        else {
            return;
        };

        for element in html_fragment.select(&icon_selector) {
            if let Some(href) = element.value().attr("href") {
                debug!(href, "Favicon href found");
                let Ok(icon_url) = self.get_href_as_absolute_url(href, url) else {
                    continue;
                };
                info!(url = icon_url.to_string(), "Favicon icon url found");
                self.icon_urls.insert(icon_url.to_string());
            }
        }
    }

    async fn set_icon_urls_from_manifests(&mut self) {
        let mut manifest_handles = HashMap::new();

        for (manifest_url, base_path_url) in self.manifest_urls.values() {
            let app_clone = self.app.clone();
            let url_clone = manifest_url.clone();
            // Spawn in parallel on main thread
            let handle = glib::spawn_future_local(async move {
                app_clone.fetch.get_as_string(url_clone.as_str()).await
            });
            manifest_handles.insert((base_path_url, manifest_url), handle);
        }

        for ((base_path_url, manifest_url), handle) in manifest_handles {
            let Ok(Ok(manifest_json)) = handle.await else {
                error!("Failed to fetch manifest: '{manifest_url}'");
                continue;
            };
            let Ok(manifest) = serde_json::from_str::<ManifestJson>(&manifest_json) else {
                continue;
            };
            let Some(icons) = manifest.icons else {
                continue;
            };
            for icon in icons {
                let Some(icon_href) = icon.src else {
                    continue;
                };
                debug!(href = icon_href, "Manifest href found");
                let Ok(icon_url) = self.get_href_as_absolute_url(&icon_href, base_path_url) else {
                    continue;
                };
                info!(icon_url = icon_url.to_string(), "Manifest icon url found");
                self.icon_urls.insert(icon_url.to_string());
            }
        }
    }

    async fn fetch_icons_from_urls(&mut self) -> Vec<(String, Rc<Icon>)> {
        let mut icon_handles = HashMap::new();
        let mut icons = Vec::new();

        for icon_url in &self.icon_urls {
            let app_clone = self.app.clone();
            let url_clone = icon_url.clone();
            // Spawn in parallel on main thread
            let handle =
                glib::spawn_future_local(
                    async move { app_clone.fetch.get_as_bytes(&url_clone).await },
                );

            icon_handles.insert(icon_url, handle);
        }

        for (url, handle) in icon_handles {
            let Ok(Ok(image_bytes)) = handle.await else {
                error!(url, "Failed to fetch image");
                continue;
            };
            let g_bytes = glib::Bytes::from(&image_bytes);
            let stream = MemoryInputStream::from_bytes(&g_bytes);
            let Ok(pixbuf) = Pixbuf::from_stream(&stream, Cancellable::NONE) else {
                error!(url, "Failed to convert image");
                continue;
            };
            let icon = Icon { pixbuf };
            icons.push((url.clone(), Rc::new(icon)));
        }

        icons
    }
}
