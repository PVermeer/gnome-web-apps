use crate::{assets, utils::strings::capitalize_all_words};
use serde::Deserialize;
use std::sync::OnceLock;
use tracing::debug;

pub static APP_ID: OnceLock<String> = OnceLock::new();
pub static VERSION: OnceLock<String> = OnceLock::new();
pub static APP_NAME: OnceLock<String> = OnceLock::new();
pub static APP_NAME_DENSE: OnceLock<String> = OnceLock::new();
pub static APP_NAME_HYPHEN: OnceLock<String> = OnceLock::new();
pub static APP_NAME_UNDERSCORE: OnceLock<String> = OnceLock::new();
pub static APP_NAME_SHORT: OnceLock<String> = OnceLock::new();
pub static APP_SUMMARY: OnceLock<String> = OnceLock::new();
pub static APP_DESCRIPTION: OnceLock<String> = OnceLock::new();
pub static APP_TEXT: OnceLock<String> = OnceLock::new();
pub static DEVELOPER: OnceLock<String> = OnceLock::new();
pub static LICENSE: OnceLock<String> = OnceLock::new();
pub static REPOSITORY: OnceLock<String> = OnceLock::new();
pub static ISSUES_URL: OnceLock<String> = OnceLock::new();
pub static BIN_NAME: OnceLock<String> = OnceLock::new();

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
struct CargoPackageBin {
    name: String,
}
#[derive(Deserialize)]
struct CargoToml {
    package: CargoPackageToml,
    bin: Vec<CargoPackageBin>,
}

static CARGO_TOML: &str = include_str!("../../app/Cargo.toml");

pub fn init() {
    set_from_cargo_toml();
    set_from_assets();
}

#[allow(unused_variables)]
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
        bin,
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
    let bin_name = bin.first().map(|bin| bin.name.clone()).unwrap_or_default();

    APP_ID.set(id).unwrap_or_default();
    VERSION.set(version).unwrap_or_default();
    APP_NAME.set(name).unwrap_or_default();
    APP_NAME_DENSE.set(name_dense).unwrap_or_default();
    APP_NAME_HYPHEN.set(name_hyphen).unwrap_or_default();
    APP_NAME_UNDERSCORE.set(name_underscore).unwrap_or_default();
    APP_NAME_SHORT.set(name_short).unwrap_or_default();
    APP_SUMMARY.set(description).unwrap_or_default();
    DEVELOPER.set(developer).unwrap_or_default();
    LICENSE.set(license).unwrap_or_default();
    REPOSITORY.set(repository).unwrap_or_default();
    ISSUES_URL.set(issues_url).unwrap_or_default();
    BIN_NAME.set(bin_name).unwrap_or_default();
}

fn set_from_assets() {
    let description = assets::get_app_description();

    APP_DESCRIPTION
        .set(description.to_string())
        .unwrap_or_default();
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
        BIN = BIN_NAME.get_value()
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
