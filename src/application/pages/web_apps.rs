mod web_app_view;

use super::NavPage;
use super::PrefNavPage;
use crate::application::App;
use crate::application::browser_configs::Browser;
use crate::application::pages::web_apps::web_app_view::WebAppView;
use crate::config;
use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use freedesktop_desktop_entry::DesktopEntry;
use libadwaita::StatusPage;
use libadwaita::{
    ActionRow, ButtonContent, NavigationPage, NavigationView, PreferencesGroup, PreferencesPage,
    gtk::{Button, Image, prelude::ButtonExt},
    prelude::{ActionRowExt, PreferencesGroupExt, PreferencesPageExt},
};
use log::debug;
use log::error;
use rand::Rng;
use rand::distributions::Alphanumeric;
use regex::Regex;
use std::borrow::Cow;
use std::cell::RefCell;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;
use url::Url;

struct DesktopFileEntries {
    name: String,
    id: String,
    browser_id: String,
    url: String,
    domain: String,
    isolate: bool,
    icon: String,
}

pub struct WebAppsPage {
    nav_page: NavigationPage,
    nav_row: ActionRow,
    nav_view: NavigationView,
    prefs_page: PreferencesPage,
}
impl NavPage for WebAppsPage {
    fn get_navpage(&self) -> &NavigationPage {
        &self.nav_page
    }

    fn get_nav_row(&self) -> Option<&ActionRow> {
        Some(&self.nav_row)
    }
}
impl WebAppsPage {
    pub fn new() -> Rc<Self> {
        let title = "Web Apps";
        let icon = "preferences-desktop-apps-symbolic";

        let PrefNavPage {
            nav_page,
            nav_row,
            nav_view,
            prefs_page,
            ..
        } = Self::build_nav_page(title, icon).with_preference_navigation_view();

        Rc::new(Self {
            nav_page,
            nav_row,
            nav_view,
            prefs_page,
        })
    }

    pub fn init(self: &Rc<Self>, app: &Rc<App>) {
        let app_section = self.clone().build_apps_section(app);
        self.prefs_page.add(&app_section);
    }

    fn build_apps_section(self: Rc<Self>, app: &Rc<App>) -> PreferencesGroup {
        let button_content = ButtonContent::builder()
            .label("New app")
            .icon_name("list-add-symbolic")
            .build();
        let new_app_button = Button::builder()
            .css_classes(["flat"])
            .child(&button_content)
            .build();
        new_app_button.connect_clicked(|_| debug!("TODO"));

        let pref_group = PreferencesGroup::builder()
            .header_suffix(&new_app_button)
            .build();

        let web_app_desktop_files = Self::get_owned_desktop_files(app);
        if web_app_desktop_files.is_empty() {
            let status_page = StatusPage::builder()
                .title("No Web Apps found")
                .description("Try adding one!")
                .icon_name("system-search-symbolic")
                .build();

            pref_group.add(&status_page);
        } else {
            for desktop_file in web_app_desktop_files {
                let web_app_row = self.clone().build_app_row(app, desktop_file);
                pref_group.add(&web_app_row);
            }
        }

        pref_group
    }

    fn build_app_row(
        self: Rc<Self>,
        app: &Rc<App>,
        desktop_file: Rc<RefCell<DesktopEntry>>,
    ) -> ActionRow {
        let desktop_file_borrow = desktop_file.borrow();

        let app_name = desktop_file_borrow
            .name(&app.desktop_file_locales)
            .unwrap_or(Cow::Borrowed("No name"));
        let app_row = ActionRow::builder()
            .title(app_name)
            .activatable(true)
            .build();

        let app_icon = Self::get_image_icon(&desktop_file_borrow);
        let suffix = Image::from_icon_name("go-next-symbolic");

        app_row.add_prefix(&app_icon);
        app_row.add_suffix(&suffix);

        drop(desktop_file_borrow);
        let app_clone = app.clone();

        app_row.connect_activated(move |_| {
            let app_page = WebAppView::new(&app_clone, &desktop_file.clone());
            app_page.init();
            self.nav_view.push(app_page.get_navpage());
        });

        app_row
    }

    fn get_applications_path(app: &Rc<App>) -> Result<PathBuf> {
        if cfg!(debug_assertions) {
            let path = Path::new("./dev-assets/desktop-files").to_path_buf();
            debug!("Using dev applications path: {}", path.display());
            return Ok(path);
        }

        let Some(data_home_path) = app.dirs.data_home.as_ref() else {
            bail!("Could not get data home path")
        };

        let path = data_home_path.join("applications");
        debug!("Using applications path: {}", path.display());

        if !path.is_dir() {
            bail!("Could not get applications path");
        }

        Ok(path)
    }

    fn get_owned_desktop_files(app: &Rc<App>) -> Vec<Rc<RefCell<DesktopEntry>>> {
        debug!("Reading user desktop files");

        let owned_web_app_key = config::DesktopFile::GWA_KEY;
        let mut owned_desktop_files = Vec::new();

        let applications_path = match Self::get_applications_path(app) {
            Err(error) => {
                error!("{error}");
                return owned_desktop_files;
            }
            Ok(path) => path,
        };

        for file in applications_path.read_dir().unwrap().flatten() {
            let Ok(desktop_file) =
                DesktopEntry::from_path(file.path(), Some(&app.desktop_file_locales))
            else {
                continue;
            };

            if desktop_file
                .desktop_entry(owned_web_app_key)
                .is_none_or(|value| value != "true")
            {
                continue;
            }

            debug!("Found desktop file: {}", desktop_file.path.display());

            owned_desktop_files.push(Rc::new(RefCell::new(desktop_file)));
        }

        owned_desktop_files
    }

    fn get_image_icon(desktop_file: &DesktopEntry) -> Image {
        let icon_name = desktop_file.icon().unwrap_or("image-missing-symbolic");
        let icon_path = Path::new(icon_name);
        if icon_path.is_file() {
            Image::from_file(icon_path)
        } else {
            Image::from_icon_name(icon_name)
        }
    }

    fn get_desktop_file_entries(
        app: &Rc<App>,
        desktop_file: &DesktopEntry,
    ) -> Result<DesktopFileEntries> {
        match (|| -> Result<DesktopFileEntries> {
            let name = desktop_file
                .name(&app.desktop_file_locales)
                .context("Missing 'Name'")?
                .to_string();

            let id = desktop_file
                .desktop_entry(config::DesktopFile::ID_KEY)
                .map(std::string::ToString::to_string)
                .or_else(|| {
                    let random_id: String = rand::thread_rng()
                        .sample_iter(&Alphanumeric)
                        .take(8)
                        .map(char::from)
                        .collect();
                    Some(random_id)
                })
                .context(format!("Missing '{}'", config::DesktopFile::ID_KEY))?;

            let browser_id = desktop_file
                .desktop_entry(config::DesktopFile::BROWSER_ID_KEY)
                .context(format!("Missing '{}'", config::DesktopFile::BROWSER_ID_KEY))?
                .to_string();

            let url = desktop_file
                .desktop_entry(config::DesktopFile::URL_KEY)
                .context(format!("Missing '{}'", config::DesktopFile::URL_KEY))?
                .to_string();

            let domain = Url::parse(&url)?
                .domain()
                .context("Failed to get domain of url")?
                .to_string();

            let isolate = desktop_file
                .desktop_entry(config::DesktopFile::ISOLATE_KEY)
                .context(format!("Missing '{}'", config::DesktopFile::ISOLATE_KEY))?
                .eq("true");

            let icon = desktop_file.icon().context("Missing 'Icon'")?.to_string();

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

    fn get_desktop_file_save_path(
        app: &Rc<App>,
        desktop_files_entries: &DesktopFileEntries,
        browser: &Browser,
    ) -> Result<PathBuf> {
        let applications_dir = Self::get_applications_path(app)?;
        let file_name = format!(
            "{}-{}{}",
            browser.file_name,
            config::APP_NAME_SHORT,
            desktop_files_entries.id
        );
        let mut desktop_file_path = applications_dir.join(file_name);
        desktop_file_path.add_extension("desktop");

        Ok(desktop_file_path)
    }

    fn get_profile_isolation_path(app: &Rc<App>, app_id: &str) -> Result<PathBuf> {
        let profiles_path = app.dirs.create_data_directory("profiles")?;
        let path = profiles_path.join(app_id);
        Ok(path)
    }

    fn save(app: &Rc<App>, desktop_file: &RefCell<DesktopEntry>) -> Result<()> {
        if let Err(error) = (|| -> Result<()> {
            let mut desktop_file = desktop_file.borrow_mut();
            let entries = Self::get_desktop_file_entries(app, &desktop_file)?;
            let browser = app
                .browsers_configs
                .get_by_id(&entries.browser_id)
                .context("Failed to get browser")?;

            let mut d_str = browser.desktop_file.clone().to_string();

            d_str = d_str.replace(
                config::DesktopFile::COMMAND_REPLACE,
                &browser.get_command()?,
            );
            d_str = d_str.replace(config::DesktopFile::NAME_REPLACE, &entries.name);
            d_str = d_str.replace(config::DesktopFile::URL_REPLACE, &entries.url);
            d_str = d_str.replace(config::DesktopFile::DOMAIN_REPLACE, &entries.domain);
            d_str = d_str.replace(config::DesktopFile::ICON_REPLACE, &entries.icon);

            let isolate_key = "is_isolated";
            let optional_isolated_value =
                Regex::new(&format!(r"%\{{{isolate_key}\s*\?\s*([^}}]+)"))
                    .unwrap()
                    .captures(&d_str)
                    .and_then(|caps| caps.get(1).map(|value| value.as_str().to_string()));

            if let Some(value) = optional_isolated_value {
                let path = Self::get_profile_isolation_path(app, &entries.id)?;
                let re = Regex::new(&format!(r"%\{{{isolate_key}\s*\?\s*[^}}]+\}}",)).unwrap();

                let replacement = if entries.isolate {
                    format!("{value}={}", path.to_string_lossy())
                } else {
                    String::new()
                };

                d_str = re.replace_all(&d_str, replacement).to_string();
            }

            let save_path = Self::get_desktop_file_save_path(app, &entries, &browser)?;
            let mut new_desktop_file =
                DesktopEntry::from_str(&save_path, &d_str, Some(&app.desktop_file_locales))?;

            new_desktop_file
                .add_desktop_entry(config::DesktopFile::GWA_KEY.to_string(), "true".to_string());
            new_desktop_file
                .add_desktop_entry(config::DesktopFile::URL_KEY.to_string(), entries.url);
            new_desktop_file.add_desktop_entry(config::DesktopFile::ID_KEY.to_string(), entries.id);
            new_desktop_file.add_desktop_entry(
                config::DesktopFile::BROWSER_ID_KEY.to_string(),
                entries.browser_id,
            );
            new_desktop_file.add_desktop_entry(
                config::DesktopFile::ISOLATE_KEY.to_string(),
                entries.isolate.to_string(),
            );

            if desktop_file.path.is_file() {
                match fs::remove_file(&desktop_file.path) {
                    Ok(()) => {}
                    Err(error) => {
                        error!("Failed to remove desktop file before saving new: {error}");
                    }
                }
            }

            debug!("Saving desktop file to: {}", save_path.display());
            fs::write(&save_path, new_desktop_file.to_string())?;
            *desktop_file = new_desktop_file;

            Ok(())
        })() {
            error!("{error}");
            bail!(error)
        }
        Ok(())
    }
}
