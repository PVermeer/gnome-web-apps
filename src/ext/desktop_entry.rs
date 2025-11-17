use crate::{application::App, config, services::browsers::Browser};
use anyhow::{Context, Result, bail};
use freedesktop_desktop_entry::DesktopEntry;
use gtk::{Image, gdk_pixbuf::Pixbuf};
use log::{debug, error};
use rand::{Rng, distributions::Alphanumeric};
use regex::Regex;
use std::{
    fs,
    path::{Path, PathBuf},
    rc::Rc,
};
use url::Url;

pub struct Icon {
    pub filename: String,
    pub pixbuf: Pixbuf,
}

pub struct DesktopFileEntries {
    name: String,
    id: String,
    browser_id: String,
    url: String,
    domain: String,
    isolate: bool,
    icon: String,
}

pub enum KeysExt {
    Gwa,
    Url,
    Id,
    BrowserId,
    Isolate,
}
impl std::fmt::Display for KeysExt {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Gwa => write!(f, "X-GWA"),
            Self::Url => write!(f, "X-GWA-URL"),
            Self::Id => write!(f, "X-GWA-ID"),
            Self::BrowserId => write!(f, "X-GWA-BROWSER-ID"),
            Self::Isolate => write!(f, "X-GWA-ISOLATE"),
        }
    }
}

pub trait DesktopEntryExt {
    fn get_entries(&self, app: &Rc<App>) -> Result<DesktopFileEntries>;
    fn save(&mut self, app: &Rc<App>) -> Result<()>;
    fn get_image_icon(&self) -> Image;
    fn set_icon(&mut self, app: &Rc<App>, icon: &Rc<Icon>) -> Result<()>;
    fn get_save_path(
        app: &Rc<App>,
        desktop_files_entries: &DesktopFileEntries,
        browser: &Browser,
    ) -> Result<PathBuf>;
    fn get_profile_path(app: &Rc<App>, app_id: &str) -> Result<PathBuf>;
}
impl DesktopEntryExt for DesktopEntry {
    fn get_entries(&self, app: &Rc<App>) -> Result<DesktopFileEntries> {
        match (|| -> Result<DesktopFileEntries> {
            let name = self
                .name(&app.desktop_file_locales)
                .context("Missing 'Name'")?
                .to_string();

            let id = self
                .desktop_entry(&KeysExt::Id.to_string())
                .map(std::string::ToString::to_string)
                .or_else(|| {
                    let random_id: String = rand::thread_rng()
                        .sample_iter(&Alphanumeric)
                        .take(8)
                        .map(char::from)
                        .collect();
                    Some(random_id)
                })
                .context(format!("Missing '{}'", KeysExt::Id))?;

            let browser_id = self
                .desktop_entry(&KeysExt::BrowserId.to_string())
                .context(format!("Missing '{}'", KeysExt::BrowserId))?
                .to_string();

            let url = self
                .desktop_entry(&KeysExt::Url.to_string())
                .context(format!("Missing '{}'", KeysExt::Url))?
                .to_string();

            let domain = Url::parse(&url)?
                .domain()
                .context("Failed to get domain of url")?
                .to_string();

            let isolate = self
                .desktop_entry(&KeysExt::Isolate.to_string())
                .context(format!("Missing '{}'", KeysExt::Isolate))?
                .eq("true");

            let icon = self.icon().context("Missing 'Icon'")?.to_string();

            Ok(DesktopFileEntries {
                name,
                id,
                browser_id,
                url,
                domain,
                isolate,
                icon,
            })
        })() {
            Ok(result) => Ok(result),
            Err(error) => {
                bail!("Failed to get all entries on desktop file: '{error}'")
            }
        }
    }

    fn save(&mut self, app: &Rc<App>) -> Result<()> {
        if let Err(error) = (|| -> Result<()> {
            let entries = self.get_entries(app)?;
            let browser = app
                .browsers_configs
                .get_by_id(&entries.browser_id)
                .context("Failed to get browser")?;

            let mut d_str = browser.desktop_file.clone().to_string();

            d_str = d_str.replace("%{command}", &browser.get_command()?);
            d_str = d_str.replace("%{name}", &entries.name);
            d_str = d_str.replace("%{url}", &entries.url);
            d_str = d_str.replace("%{domain}", &entries.domain);
            d_str = d_str.replace("%{icon}", &entries.icon);

            let isolate_key = "is_isolated";
            let optional_isolated_value =
                Regex::new(&format!(r"%\{{{isolate_key}\s*\?\s*([^}}]+)"))
                    .unwrap()
                    .captures(&d_str)
                    .and_then(|caps| caps.get(1).map(|value| value.as_str().to_string()));

            if let Some(value) = optional_isolated_value {
                let path = Self::get_profile_path(app, &entries.id)?;
                let re = Regex::new(&format!(r"%\{{{isolate_key}\s*\?\s*[^}}]+\}}",)).unwrap();

                let replacement = if entries.isolate {
                    format!("{value}={}", path.to_string_lossy())
                } else {
                    String::new()
                };

                d_str = re.replace_all(&d_str, replacement).to_string();
            }

            let save_path = Self::get_save_path(app, &entries, &browser)?;
            let mut new_desktop_file =
                DesktopEntry::from_str(&save_path, &d_str, Some(&app.desktop_file_locales))?;

            new_desktop_file.add_desktop_entry(KeysExt::Gwa.to_string(), "true".to_string());
            new_desktop_file.add_desktop_entry(KeysExt::Url.to_string(), entries.url);
            new_desktop_file.add_desktop_entry(KeysExt::Id.to_string(), entries.id);
            new_desktop_file.add_desktop_entry(KeysExt::BrowserId.to_string(), entries.browser_id);
            new_desktop_file
                .add_desktop_entry(KeysExt::Isolate.to_string(), entries.isolate.to_string());

            if self.path.is_file() {
                match fs::remove_file(&self.path) {
                    Ok(()) => {}
                    Err(error) => {
                        error!("Failed to remove desktop file before saving new: {error}");
                    }
                }
            }

            debug!("Saving desktop file to: {}", save_path.display());
            fs::write(&save_path, new_desktop_file.to_string())?;
            *self = new_desktop_file;

            Ok(())
        })() {
            error!("{error}");
            bail!(error)
        }
        Ok(())
    }

    fn get_image_icon(&self) -> Image {
        let icon_name = self.icon().unwrap_or("image-missing-symbolic");
        let icon_path = Path::new(icon_name);
        if icon_path.is_file() {
            Image::from_file(icon_path)
        } else {
            Image::from_icon_name(icon_name)
        }
    }

    fn set_icon(&mut self, app: &Rc<App>, icon: &Rc<Icon>) -> Result<()> {
        let app_id = self
            .desktop_entry(&KeysExt::Id.to_string())
            .context("No app id on desktop file!")?;

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

        let icon_dir = app.get_icons_dir()?;
        let icon_name = sanitize_filename::sanitize(format!("{app_id}-{filename}"));
        let save_path = icon_dir.join(&icon_name);

        debug!("Saving {} to fs: {}", &icon_name, save_path.display());
        app.dirs
            .place_data_file(&save_path)
            .context("Failed to create paths")?;
        icon.pixbuf
            .savev(save_path.clone(), "png", &[])
            .context("Failed to save icon to fs")?;

        self.add_desktop_entry(
            "Icon".to_string(),
            save_path
                .to_str()
                .context("Cannot convert icon path to string")?
                .to_string(),
        );

        debug!(
            "Set a new 'Icon' on `desktop file`: {}",
            &self.desktop_entry("Icon").unwrap_or_default()
        );
        Ok(())
    }

    fn get_save_path(
        app: &Rc<App>,
        desktop_files_entries: &DesktopFileEntries,
        browser: &Browser,
    ) -> Result<PathBuf> {
        let applications_dir = app.get_applications_dir()?;
        let file_name = format!(
            "{}-{}{}",
            browser.desktop_file_name_prefix,
            config::APP_NAME_SHORT,
            desktop_files_entries.id
        );
        let mut desktop_file_path = applications_dir.join(file_name);
        desktop_file_path.add_extension("desktop");

        Ok(desktop_file_path)
    }

    fn get_profile_path(app: &Rc<App>, app_id: &str) -> Result<PathBuf> {
        let profiles_path = app.dirs.create_data_directory("profiles")?;
        let path = profiles_path.join(app_id);
        Ok(path)
    }
}
