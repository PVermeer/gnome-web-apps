use std::sync::OnceLock;

use libadwaita::gtk::License;
use serde::Deserialize;
use tracing::debug;

use crate::services::utils::strings::capitalize_all_words;

pub static APP_ID: OnceLock<String> = OnceLock::new();
pub static VERSION: OnceLock<String> = OnceLock::new();
pub static APP_NAME: OnceLock<String> = OnceLock::new();
pub static APP_NAME_HYPHEN: OnceLock<String> = OnceLock::new();
pub static APP_NAME_UNDERSCORE: OnceLock<String> = OnceLock::new();
pub static APP_NAME_SHORT: OnceLock<String> = OnceLock::new();
pub static DEVELOPER: OnceLock<String> = OnceLock::new();
pub static LICENSE: OnceLock<License> = OnceLock::new();
pub static ISSUES_URL: OnceLock<String> = OnceLock::new();

#[derive(Deserialize)]
struct CargoPackageToml {
    name: String,
    version: String,
    license: String,
    authors: Vec<String>,
    repository: String,
    homepage: String,
    documentation: String,
}
#[derive(Deserialize)]
struct CargoToml {
    package: CargoPackageToml,
}

static CARGO_TOML: &str = include_str!("../../Cargo.toml");

#[allow(unused)]
pub fn init() {
    let CargoToml {
        package:
            CargoPackageToml {
                name,
                version,
                license,
                authors,
                repository,
                homepage,
                documentation,
            },
    } = toml::from_str(CARGO_TOML).expect("Could not load Cargo.toml");

    let name_hyphen = name.clone();
    let name_underscore = name.replace('-', "_");
    let name = capitalize_all_words(&name_hyphen.replace('-', " "));
    let name_dense = name.replace(' ', "");
    let name_short = name
        .split_whitespace()
        .map(|word| word.chars().next().unwrap_or_default())
        .collect::<String>()
        .to_lowercase();

    let id = format!("org.pvermeer.{name_dense}");
    let license = match license.as_str() {
        "GPL-3.0" => License::Gpl30,
        "GPL-3.0-only" => License::Gpl30Only,
        _ => panic!("Could not convert license"),
    };
    let developer = authors
        .first()
        .expect("Could not load developer / author")
        .clone();
    let issues_url = format!("{repository}/issues");

    APP_ID.set(id);
    VERSION.set(version);
    APP_NAME.set(name);
    APP_NAME_HYPHEN.set(name_hyphen);
    APP_NAME_UNDERSCORE.set(name_underscore);
    APP_NAME_SHORT.set(name_short);
    DEVELOPER.set(developer);
    LICENSE.set(license);
    ISSUES_URL.set(issues_url);
}

pub fn log_all_values_debug() {
    debug!(
        APP_ID = APP_ID.get_value(),
        VERSION = VERSION.get_value(),
        APP_NAME = APP_NAME.get_value(),
        APP_NAME_HYPHEN = APP_NAME_HYPHEN.get_value(),
        APP_NAME_UNDERSCORE = APP_NAME_UNDERSCORE.get_value(),
        APP_NAME_SHORT = APP_NAME_SHORT.get_value(),
        DEVELOPER = DEVELOPER.get_value(),
        LICENSE = format!("{:?}", LICENSE.get_value()),
    );
}

pub trait OnceLockExt<T> {
    fn get_value(&self) -> &T;
}
impl<T> OnceLockExt<T> for OnceLock<T> {
    fn get_value(&self) -> &T {
        self.get().unwrap()
    }
}
