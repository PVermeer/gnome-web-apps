use crate::{
    app_dirs::AppDirs,
    browsers::{Browser, BrowserConfigs},
    config::{self, OnceLockExt},
};
use anyhow::{Context, Result, bail};
use freedesktop_desktop_entry::DesktopEntry;
use gtk::{Image, gdk_pixbuf::Pixbuf, prelude::WidgetExt};
use rand::{Rng, distributions::Alphanumeric};
use regex::Regex;
use std::{
    fmt::Display,
    fs::{self},
    path::{Path, PathBuf},
    rc::Rc,
};
use tracing::{debug, error, info};
use url::Url;
use validator::ValidateUrl;

pub struct Icon {
    pub pixbuf: Pixbuf,
}

pub struct DesktopFileEntries {
    name: String,
    app_id: String,
    browser: Rc<Browser>,
    url: String,
    domain: String,
    isolate: bool,
    icon: PathBuf,
    profile_path: String,
}

#[allow(unused)]
enum Keys {
    Gwa,
    Url,
    Id,
    BrowserId,
    Isolate,
    Profile,
    Name,
    Exec,
    Icon,
    StartupWMClass,
}
impl Display for Keys {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let identifier = config::APP_NAME_SHORT.get_value().to_uppercase();

        match self {
            Self::Gwa => write!(f, "X-{}", &identifier),
            Self::Id => write!(f, "X-{}-ID", &identifier),
            Self::Url => write!(f, "X-{}-URL", &identifier),
            Self::BrowserId => write!(f, "X-{}-BROWSER-ID", &identifier),
            Self::Isolate => write!(f, "X-{}-ISOLATE", &identifier),
            Self::Profile => write!(f, "X-{}-PROFILE", &identifier),
            Self::Name => write!(f, "Name"),
            Self::Exec => write!(f, "Exec"),
            Self::Icon => write!(f, "Icon"),
            Self::StartupWMClass => write!(f, "StartupWMClass"),
        }
    }
}

fn map_to_string_option(value: &str) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn map_to_bool_option(value: &str) -> Option<bool> {
    if value.is_empty() {
        None
    } else {
        Some(value.eq("true"))
    }
}

fn map_to_path_option(value: PathBuf) -> Option<PathBuf> {
    if value.as_os_str().is_empty() {
        None
    } else {
        Some(value)
    }
}

#[derive(Clone)]
pub struct DesktopFile {
    desktop_entry: DesktopEntry,
    browser_configs: Rc<BrowserConfigs>,
    app_dirs: Rc<AppDirs>,
}
impl DesktopFile {
    pub fn new(browser_configs: &Rc<BrowserConfigs>, app_dirs: &Rc<AppDirs>) -> Self {
        let mut desktop_entry = DesktopEntry::from_appid(String::new());

        let random_id: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(8)
            .map(char::from)
            .collect();
        desktop_entry.add_desktop_entry(Keys::Id.to_string(), random_id);

        Self {
            desktop_entry,
            browser_configs: browser_configs.clone(),
            app_dirs: app_dirs.clone(),
        }
    }

    pub fn from_path(
        path: &Path,
        browser_configs: &Rc<BrowserConfigs>,
        app_dirs: &Rc<AppDirs>,
    ) -> Result<Self> {
        let desktop_entry = DesktopEntry::from_path(path, None::<&[String]>)?;

        Ok(Self {
            desktop_entry,
            browser_configs: browser_configs.clone(),
            app_dirs: app_dirs.clone(),
        })
    }

    pub fn from_string(
        path: &Path,
        str: &str,
        browser_configs: &Rc<BrowserConfigs>,
        app_dirs: &Rc<AppDirs>,
    ) -> Result<Self> {
        let desktop_entry = DesktopEntry::from_str(path, str, None::<&[String]>)?;

        Ok(Self {
            desktop_entry,
            browser_configs: browser_configs.clone(),
            app_dirs: app_dirs.clone(),
        })
    }

    pub fn get_path(&self) -> PathBuf {
        self.desktop_entry.path.clone()
    }

    pub fn set_path(&mut self, path: &Path) {
        self.desktop_entry.path = path.to_path_buf();

        debug!("Set a new 'path' for desktop file: {}", path.display());
    }

    pub fn get_is_owned_app(&self) -> bool {
        self.desktop_entry
            .desktop_entry(&Keys::Gwa.to_string())
            .and_then(map_to_bool_option)
            .is_some_and(|is_owned| is_owned)
    }

    pub fn set_is_owned_app(&mut self) {
        self.desktop_entry
            .add_desktop_entry(Keys::Gwa.to_string(), true.to_string());

        debug!(
            "Set '{}' on desktop file: {}",
            &Keys::Gwa.to_string(),
            &self
                .desktop_entry
                .desktop_entry(&Keys::Gwa.to_string())
                .unwrap_or_default()
        );
    }

    pub fn get_name(&self) -> Option<String> {
        self.desktop_entry
            .desktop_entry(&Keys::Name.to_string())
            .and_then(map_to_string_option)
    }

    pub fn set_name(&mut self, id: &str) {
        self.desktop_entry
            .add_desktop_entry(Keys::Name.to_string(), id.to_string());

        debug!(
            "Set '{}' on desktop file: {}",
            &Keys::Name.to_string(),
            &self
                .desktop_entry
                .desktop_entry(&Keys::Name.to_string())
                .unwrap_or_default()
        );
    }

    pub fn get_exec(&self) -> Option<String> {
        self.desktop_entry
            .desktop_entry(&Keys::Exec.to_string())
            .and_then(map_to_string_option)
    }

    pub fn get_id(&self) -> Option<String> {
        self.desktop_entry
            .desktop_entry(&Keys::Id.to_string())
            .and_then(map_to_string_option)
    }

    pub fn set_id(&mut self, id: &str) {
        self.desktop_entry
            .add_desktop_entry(Keys::Id.to_string(), id.to_string());

        debug!(
            "Set '{}' on desktop file: {}",
            &Keys::Id.to_string(),
            &self
                .desktop_entry
                .desktop_entry(&Keys::Id.to_string())
                .unwrap_or_default()
        );
    }

    pub fn get_url(&self) -> Option<String> {
        self.desktop_entry
            .desktop_entry(&Keys::Url.to_string())
            .and_then(map_to_string_option)
    }

    pub fn set_url(&mut self, url: &str) {
        self.desktop_entry
            .add_desktop_entry(Keys::Url.to_string(), url.to_string());

        debug!(
            "Set '{}' on desktop file: {}",
            &Keys::Url.to_string(),
            &self
                .desktop_entry
                .desktop_entry(&Keys::Url.to_string())
                .unwrap_or_default()
        );
    }

    pub fn get_browser(&self) -> Option<Rc<Browser>> {
        self.desktop_entry
            .desktop_entry(&Keys::BrowserId.to_string())
            .and_then(map_to_string_option)
            .and_then(|browser_id| self.browser_configs.get_by_id(&browser_id))
    }

    pub fn set_browser(&mut self, browser: &Rc<Browser>) {
        self.desktop_entry
            .add_desktop_entry(Keys::BrowserId.to_string(), browser.id.clone());

        debug!(
            "Set '{}' on desktop file: {}",
            &Keys::BrowserId.to_string(),
            &self
                .desktop_entry
                .desktop_entry(&Keys::BrowserId.to_string())
                .unwrap_or_default()
        );
    }

    pub fn get_isolated(&self) -> Option<bool> {
        self.desktop_entry
            .desktop_entry(&Keys::Isolate.to_string())
            .and_then(map_to_bool_option)
    }

    pub fn set_isolated(&mut self, is_isolated: bool) {
        self.desktop_entry
            .add_desktop_entry(Keys::Isolate.to_string(), is_isolated.to_string());

        debug!(
            "Set '{}' on desktop file: {}",
            &Keys::Isolate.to_string(),
            &self
                .desktop_entry
                .desktop_entry(&Keys::Isolate.to_string())
                .unwrap_or_default()
        );
    }

    pub fn get_icon(&self) -> Image {
        let fallback_icon = "image-missing-symbolic";
        let icon_name = self.desktop_entry.icon().unwrap_or_default();
        let icon_path = Path::new(icon_name);
        if icon_path.is_file() {
            Image::from_file(icon_path)
        } else if !icon_name.is_empty() {
            Image::from_icon_name(icon_name)
        } else {
            let image = Image::from_icon_name(fallback_icon);
            image.add_css_class("error");
            image
        }
    }

    pub fn get_icon_path(&self) -> Option<PathBuf> {
        self.desktop_entry
            .desktop_entry(&Keys::Icon.to_string())
            .map(|str| Path::new(str).to_path_buf())
            .and_then(map_to_path_option)
    }

    pub fn set_icon_path(&mut self, path: &Path) {
        self.desktop_entry
            .add_desktop_entry(Keys::Icon.to_string(), path.to_string_lossy().to_string());

        debug!(
            "Set '{}' on desktop file: {}",
            &Keys::Icon.to_string(),
            &self
                .desktop_entry
                .desktop_entry(&Keys::Icon.to_string())
                .unwrap_or_default()
        );
    }

    pub fn get_profile_path(&self) -> Option<String> {
        self.desktop_entry
            .desktop_entry(&Keys::Profile.to_string())
            .and_then(map_to_string_option)
    }

    pub fn set_profile_path(&mut self, path: &str) {
        self.desktop_entry
            .add_desktop_entry(Keys::Profile.to_string(), path.to_string());

        debug!(
            "Set '{}' on desktop file: {}",
            &Keys::Profile.to_string(),
            &self
                .desktop_entry
                .desktop_entry(&Keys::Profile.to_string())
                .unwrap_or_default()
        );
    }

    pub fn build_profile_path(&self) -> Result<String> {
        let browser = self.get_browser().context("No browser on 'DesktopFile'")?;
        let is_isolated = self.get_isolated().unwrap_or(false);

        if !is_isolated {
            bail!("Isolate is not set")
        }
        if !browser.can_isolate {
            bail!("Browser cannot isolate")
        }

        let id = self.get_id().context("No id on 'DesktopFile'")?;
        let profile_path = browser.get_profile_path(&id)?;

        debug!("Using profile path: {}", &profile_path);

        Ok(profile_path)
    }

    pub fn validate(&self) -> Result<()> {
        match self.to_new_from_browser() {
            Err(error) => {
                debug!("Validate error: {error:?}");
                Err(error)
            }
            Ok(_) => Ok(()),
        }
    }

    pub fn save(&mut self) -> Result<()> {
        if let Err(error) = (|| -> Result<()> {
            let new_desktop_file = self.to_new_from_browser()?;

            if self.desktop_entry.path.is_file() && !self.desktop_entry.path.is_symlink() {
                match fs::remove_file(&self.desktop_entry.path) {
                    Ok(()) => {}
                    Err(error) => {
                        error!("Failed to remove desktop file before saving new: {error:?}");
                    }
                }
            }

            let save_path = new_desktop_file.desktop_entry.path.clone();

            debug!("Saving desktop file to: {}", save_path.display());
            fs::write(&save_path, new_desktop_file.desktop_entry.to_string())?;
            self.desktop_entry = new_desktop_file.desktop_entry;

            Ok(())
        })() {
            error!("{error:?}");
            bail!(error)
        }
        Ok(())
    }

    pub fn delete(&self) -> Result<()> {
        let mut is_error = false;

        if self.desktop_entry.path.is_file() {
            match fs::remove_file(&self.desktop_entry.path) {
                Ok(()) => {}
                Err(error) => {
                    error!("Failed to remove desktop file: {error:?}");
                    is_error = true;
                }
            }
        }

        if let Some(icon_path) = self.get_icon_path()
            && icon_path.is_file()
        {
            match fs::remove_file(icon_path) {
                Ok(()) => {}
                Err(error) => {
                    error!("Failed to remove icon file: {error:?}");
                    is_error = true;
                }
            }
        }

        if let Some(profile_path) = self.get_profile_path()
            && Path::new(&profile_path).is_dir()
        {
            match fs::remove_dir_all(profile_path) {
                Ok(()) => {}
                Err(error) => {
                    error!("Failed to remove profile: {error:?}");
                    is_error = true;
                }
            }
        }

        if is_error {
            bail!("Some files could not be removed, check logs")
        }

        info!(
            "Succesfully removed web app: {}",
            self.get_name().unwrap_or_default()
        );
        Ok(())
    }

    fn get_entries(&self) -> Result<DesktopFileEntries> {
        match (|| -> Result<DesktopFileEntries> {
            let name = self.get_name().context("Missing 'Name'")?;
            let app_id = self.get_id().context(format!("Missing '{}'", Keys::Id))?;
            let url = self
                .get_url()
                .context(format!("Missing '{}'", Keys::Url))
                .ok()
                .filter(ValidateUrl::validate_url)
                .context(format!("Invalid '{}'", Keys::Url))?;
            let browser = self
                .get_browser()
                .context(format!("Missing '{}'", Keys::BrowserId))?;
            let domain = Url::parse(&url)?
                .domain()
                .and_then(map_to_string_option)
                .context("Failed to get domain of url")?;
            let isolate = self
                .get_isolated()
                .context(format!("Missing '{}'", Keys::Isolate))?;
            let icon = self
                .get_icon_path()
                .and_then(map_to_path_option)
                .context("Missing 'Icon'")?;
            let profile_path = self
                .get_profile_path()
                .or_else(|| {
                    if isolate {
                        None
                    } else {
                        Some(String::default())
                    }
                })
                .context(format!("Missing '{}'", Keys::Profile))?;

            Ok(DesktopFileEntries {
                name,
                app_id,
                browser,
                url,
                domain,
                isolate,
                icon,
                profile_path,
            })
        })() {
            Ok(result) => Ok(result),
            Err(error) => {
                bail!("Failed to get all entries on desktop file: '{error:?}'")
            }
        }
    }

    fn get_save_path(&self) -> Result<PathBuf> {
        let applications_dir = self.app_dirs.applications();
        let file_name = format!(
            "{}-{}-{}",
            self.get_browser()
                .context("Failed to get browser")?
                .desktop_file_name_prefix,
            config::APP_NAME_SHORT.get_value(),
            self.get_id().context("Failed to get my id")?
        );
        let mut desktop_file_path = applications_dir.join(file_name);
        desktop_file_path.add_extension("desktop");

        Ok(desktop_file_path)
    }

    fn to_new_from_browser(&self) -> Result<DesktopFile> {
        let entries = self.get_entries()?;
        let save_path = self.get_save_path()?;
        let app_name_short = config::APP_NAME_SHORT.get_value();
        let app_id = format!("{}-{}", app_name_short, entries.app_id);

        let mut d_str = entries.browser.desktop_file.clone().to_string();
        d_str = d_str.replace("%{command}", &entries.browser.get_command()?);
        d_str = d_str.replace("%{name}", &entries.name);
        d_str = d_str.replace("%{url}", &entries.url);
        d_str = d_str.replace("%{domain}", &entries.domain);
        d_str = d_str.replace("%{icon}", &entries.icon.to_string_lossy());
        d_str = d_str.replace("%{app_id}", &app_id);

        let isolate_key = "is_isolated";
        let optional_isolated_value = Regex::new(&format!(r"%\{{{isolate_key}\s*\?\s*([^}}]+)"))
            .unwrap()
            .captures(&d_str)
            .and_then(|caps| caps.get(1).map(|value| value.as_str().to_string()));

        if let Some(value) = optional_isolated_value {
            let re = Regex::new(&format!(r"%\{{{isolate_key}\s*\?\s*[^}}]+\}}",)).unwrap();

            let replacement = if entries.isolate {
                format!("{value}={}", entries.profile_path)
            } else {
                String::new()
            };

            d_str = re.replace_all(&d_str, replacement).to_string();
        }

        let mut new_desktop_file =
            Self::from_string(&save_path, &d_str, &self.browser_configs, &self.app_dirs)?;

        new_desktop_file.set_is_owned_app();
        new_desktop_file.set_url(&entries.url);
        new_desktop_file.set_id(&entries.app_id);
        new_desktop_file.set_browser(&entries.browser);
        new_desktop_file.set_isolated(entries.isolate);
        new_desktop_file.set_profile_path(&entries.profile_path);

        Ok(new_desktop_file)
    }
}
impl std::fmt::Display for DesktopFile {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.desktop_entry.fmt(f)
    }
}
