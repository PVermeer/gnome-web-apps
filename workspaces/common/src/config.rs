use crate::utils::strings::capitalize_all_words;
use serde::Deserialize;
use std::sync::OnceLock;
use tracing::debug;

pub static APP_ID: OnceLock<String> = OnceLock::new();
pub static VERSION: OnceLock<String> = OnceLock::new();
pub static APP_NAME: OnceLock<String> = OnceLock::new();
pub static APP_DESCRIPTION: OnceLock<String> = OnceLock::new();
pub static APP_NAME_HYPHEN: OnceLock<String> = OnceLock::new();
pub static APP_NAME_UNDERSCORE: OnceLock<String> = OnceLock::new();
pub static APP_NAME_SHORT: OnceLock<String> = OnceLock::new();
pub static DEVELOPER: OnceLock<String> = OnceLock::new();
pub static LICENSE: OnceLock<String> = OnceLock::new();
pub static ISSUES_URL: OnceLock<String> = OnceLock::new();

#[derive(Deserialize)]
struct CargoPackageToml {
    name: String,
    description: String,
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

static CARGO_TOML: &str = include_str!("../../app/Cargo.toml");

pub fn init() {
    set_from_cargo_toml();
}

#[allow(unused)]
fn set_from_cargo_toml() {
    let CargoToml {
        package:
            CargoPackageToml {
                name,
                description,
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
    let developer = authors
        .first()
        .expect("Could not load developer / author")
        .clone();
    let issues_url = format!("{repository}/issues");

    APP_ID.set(id);
    VERSION.set(version);
    APP_NAME.set(name);
    APP_DESCRIPTION.set(description);
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
