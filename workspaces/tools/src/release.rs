use anyhow::{Result, bail};
use chrono::DateTime;
use clap::Parser;
use common::{
    assets,
    config::{self, OnceLockExt},
    utils,
};
use freedesktop_desktop_entry::DesktopEntry;
use git_cliff::args::Opt;
use semver::Version;
use std::{fmt::Write as _, io::Write};
use std::{
    fs::{self, File},
    path::{Path, PathBuf},
    process::Command,
};
use tracing::{Level, error, info};
use tracing_subscriber::{FmtSubscriber, util::SubscriberInitExt};

static FLATPAK_MANIFEST_IN: &str = include_str!("../../../flatpak/manifest.in");

fn main() -> Result<()> {
    /* Logging */
    let mut log_level = if cfg!(debug_assertions) {
        Level::DEBUG
    } else {
        Level::INFO
    };
    log_level = utils::env::get_log_level().unwrap_or(log_level);
    let logger = FmtSubscriber::builder()
        .without_time()
        .with_target(false)
        .with_max_level(log_level)
        .finish();
    logger.init();

    config::init();
    config::log_all_values_debug();

    create_app_desktop_file()?;
    create_app_icon()?;

    update_flatpak_manifest()?;
    let releases_xml = generate_changelog()?;
    create_app_metainfo_file(&releases_xml)?;

    Ok(())
}

fn create_app_desktop_file() -> Result<()> {
    info!("==== Creating app desktop file");

    let desktop_file = assets::get_desktop_file();
    let app_id = config::APP_ID.get_value();
    let app_name = config::APP_NAME.get_value();
    let bin_name = config::BIN_NAME.get_value();
    let file_name = desktop_file_name();
    let save_dir = assets_path().join("desktop");
    let save_path = save_dir.join(file_name);

    let mut base_desktop_file =
        DesktopEntry::from_str(&save_path, desktop_file, None::<&[String]>)?;

    base_desktop_file.add_desktop_entry("Name".to_string(), app_name.clone());
    base_desktop_file.add_desktop_entry("Icon".to_string(), app_id.clone());
    base_desktop_file.add_desktop_entry("StartupWMClass".to_string(), app_id.clone());
    base_desktop_file.add_desktop_entry("Exec".to_string(), bin_name.clone());

    if !save_dir.is_dir() {
        fs::create_dir_all(&save_dir)?;
    }

    fs::write(&save_path, base_desktop_file.to_string()).inspect_err(|err| {
        error!(
            error = err.to_string(),
            path = &save_path.to_string_lossy().to_string(),
            "Failed to save desktop file"
        );
    })?;

    info!(
        desktop_file = &save_path.to_string_lossy().to_string(),
        "Created desktop file:"
    );

    Ok(())
}

fn create_app_icon() -> Result<()> {
    info!("==== Creating app icon");

    let file_name = icon_file_name();
    let save_dir = assets_path().join("desktop");
    let save_path = save_dir.join(file_name);

    if !save_dir.is_dir() {
        fs::create_dir_all(&save_dir)?;
    }

    let mut icon_file = File::create(&save_path)?;
    icon_file
        .write_all(assets::get_icon_data())
        .inspect_err(|err| {
            error!(
                error = err.to_string(),
                path = &save_path.to_string_lossy().to_string(),
                "Failed to save flatpak manifest"
            );
        })?;

    info!(
        app_icon = &save_path.to_string_lossy().to_string(),
        "Created app icon:"
    );

    Ok(())
}

fn update_flatpak_manifest() -> Result<()> {
    info!("==== Updating flatpak manifest");

    let save_dir = project_path().join("flatpak");
    let app_id = config::APP_ID.get_value();
    let app_name = config::APP_NAME.get_value();
    let app_name_dense = config::APP_NAME_DENSE.get_value();
    let app_name_short = config::APP_NAME_SHORT.get_value();
    let app_name_hyphen = config::APP_NAME_HYPHEN.get_value();
    let bin_name = config::BIN_NAME.get_value();
    let git_repository = &format!("{}.git", config::REPOSITORY.get_value());
    let git_tag = &format!("v{}", config::VERSION.get_value());

    let mut manifest = FLATPAK_MANIFEST_IN.to_string();
    manifest = manifest.replace("%{app_id}", app_id);
    manifest = manifest.replace("%{app_name}", app_name);
    manifest = manifest.replace("%{app_name_dense}", app_name_dense);
    manifest = manifest.replace("%{app_name_short}", app_name_short);
    manifest = manifest.replace("%{app_name_hyphen}", app_name_hyphen);
    manifest = manifest.replace("%{bin_name}", bin_name);

    let mut manifest_dev = manifest.clone();
    manifest_dev = manifest_dev.replace("%{sources_type}", "dir");
    manifest_dev = manifest_dev.replace("%{sources_location}", "path: ..");
    manifest_dev = manifest_dev.replace("%{git_tag}", "");
    manifest_dev = manifest_dev.replace("%{cargo_sources}", "");
    manifest_dev = manifest_dev.replace("%{cargo_home}", "flatpak");

    let save_path_dev = &save_dir.join(format!("{app_id}.Devel.yml"));
    fs::write(save_path_dev, &manifest_dev).inspect_err(|err| {
        error!(
            path = save_path_dev.to_string_lossy().to_string(),
            error = err.to_string(),
            "Failed to save flatpak manifest-Devel"
        );
    })?;

    manifest = manifest.replace("%{sources_type}", "git");
    manifest = manifest.replace("%{sources_location}", &format!("url: {git_repository}"));
    manifest = manifest.replace("%{git_tag}", &format!("tag: {git_tag}"));
    manifest = manifest.replace("%{cargo_sources}", "- cargo-sources.json");
    manifest = manifest.replace("%{cargo_home}", "cargo");

    let save_path = &save_dir.join(format!("{app_id}.yml"));

    fs::write(save_path, &manifest).inspect_err(|err| {
        error!(
            error = err.to_string(),
            path = save_path.to_string_lossy().to_string(),
            "Failed to save flatpak manifest"
        );
    })?;

    info!(
        flathub = save_path.to_string_lossy().to_string(),
        dev = save_path_dev.to_string_lossy().to_string(),
        "Updated flatpak manifests:"
    );

    Ok(())
}

#[allow(clippy::too_many_lines)] // No exports of types from git_cliff...
fn generate_changelog() -> Result<String> {
    info!("==== Generating changelogs");

    let changelog_path = &project_path().join("CHANGELOG.md");
    let mut changelog_file = &File::create(changelog_path)?;
    let app_version = Version::parse(config::VERSION.get_value())?;
    let mut git_cliff_args = Opt::parse();
    git_cliff_args.tag = Some(app_version.to_string());
    git_cliff_args.config = project_path()
        .join("workspaces")
        .join("tools")
        .join("git-cliff.toml");
    let mut changelog = git_cliff::run(git_cliff_args.clone())?;

    let Some(Ok(last_released_version)) = changelog.releases.last().and_then(|release| {
        release
            .version
            .clone()
            .map(|version| Version::parse(&version[1..]))
    }) else {
        bail!("No latest release version found in git");
    };

    if last_released_version >= app_version {
        info!("No new version detected, so not creating new changelogs");
        return Ok(String::new());
    }

    // Remove initial release
    changelog.releases.pop();

    let last_n_releases = if changelog.releases.len() < 5 {
        changelog.releases.len()
    } else {
        5
    };
    let _ = changelog.releases.split_off(last_n_releases);

    changelog.generate(&mut changelog_file)?;

    info!(
        changelog = changelog_path.to_string_lossy().to_string(),
        "Written new changelog:"
    );

    // === Start of metainfo.xml parsing

    let mut all_releases_xml = String::new();
    // changelog.releases.reverse();

    for release in changelog.releases {
        let Some(version) = release.version else {
            bail!("No version found for release")
        };
        let Some(timestamp) = release.timestamp else {
            bail!("No date found for release")
        };
        let Some(date_time) = DateTime::from_timestamp(timestamp, 0) else {
            bail!("Could not convert timestamp to date")
        };
        let date = date_time.date_naive().to_string();

        let mut release_xml = String::new();
        let _ = write!(
            release_xml,
            r#"
    <release version="{version}" date="{date}">
      <description>"#
        );

        let mut features = Vec::new();
        let mut fixes = Vec::new();

        for commit in &release.commits {
            let Some(conventional_commit) = &commit.conv else {
                continue;
            };
            let commit_type = conventional_commit.type_().as_str();
            match commit_type {
                "feat" => features.push(conventional_commit),
                "fix" => fixes.push(conventional_commit),
                _ => (),
            }
        }

        if !features.is_empty() {
            let _ = write!(
                release_xml,
                r"
        <p>New features:</p>
        <ul>"
            );

            for feat in &features {
                let scope = feat
                    .scope()
                    .map(|scope| format!("{}: ", scope.as_str()))
                    .unwrap_or_default();
                let feature_message = &feat.description();
                let _ = write!(
                    release_xml,
                    r"
          <li>{scope}{feature_message}</li>"
                );
            }

            let _ = write!(
                release_xml,
                r"
        </ul>"
            );
        }

        if !fixes.is_empty() {
            let _ = write!(
                release_xml,
                r"
        <p>Fixes:</p>
        <ul>"
            );

            for fix in &fixes {
                let scope = fix
                    .scope()
                    .map(|scope| format!("{}: ", scope.as_str()))
                    .unwrap_or_default();
                let feature_message = &fix.description();
                let _ = write!(
                    release_xml,
                    r"
          <li>{scope}{feature_message}</li>"
                );
            }

            let _ = write!(
                release_xml,
                r"
        </ul>"
            );
        }

        if features.is_empty() && fixes.is_empty() {
            let _ = write!(
                release_xml,
                r"
        <p>No notable changes</p>"
            );
        }

        let _ = write!(
            release_xml,
            r"
      </description>
    </release>
    "
        );

        let _ = write!(all_releases_xml, "{release_xml}");
    }

    Ok(all_releases_xml)
}

fn create_app_metainfo_file(releases_xml: &str) -> Result<()> {
    info!("==== Creating metainfo.xml");

    let app_id = config::APP_ID.get_value();
    let app_name = config::APP_NAME.get_value();
    let app_name_hyphen = config::APP_NAME_HYPHEN.get_value();
    let developer = config::DEVELOPER.get_value();
    let developer_id = &developer.to_lowercase();
    let app_summary = config::APP_SUMMARY.get_value();
    let app_description = config::APP_DESCRIPTION.get_value();
    let license = config::LICENSE.get_value();
    let repository = config::REPOSITORY.get_value();
    let assets_path = assets_path();
    let git_tag = format!("v{}", config::VERSION.get_value());

    let screenshot_base_url = &format!(
        "https://raw.githubusercontent.com/{developer_id}/{app_name_hyphen}/refs/tags/{git_tag}/assets/screenshots"
    );
    let mut i = 0;
    let screenshots = utils::files::get_entries_in_dir(&assets_path.join("screenshots"))?
        .iter()
        .map(|file| {
            let Some(caption) = file
                .path()
                .file_stem()
                .map(|file_stem| file_stem.to_string_lossy())
                .and_then(|file_stem| {
                    file_stem
                        .split_once('-')
                        .map(|(_, caption)| caption.to_string())
                })
            else {
                return String::new();
            };

            let default_screenshot = if i == 0 { " type=\"default\"" } else { "" };
            let screenshot_xml = format!(
                r"
    <screenshot{default_screenshot}>
      <image>{screenshot_base_url}/{}</image>
      <caption>{caption}</caption>
    </screenshot>",
                file.file_name().display()
            );

            i += 1;
            screenshot_xml
        })
        .collect::<Vec<String>>()
        .join("\n");

    let mut meta_data = assets::get_meta_info().to_string();
    meta_data = meta_data.replace("%{app_id}", app_id);
    meta_data = meta_data.replace("%{app_name}", app_name);
    meta_data = meta_data.replace("%{developer}", developer);
    meta_data = meta_data.replace("%{developer_id}", developer_id);
    meta_data = meta_data.replace("%{app_summary}", app_summary);
    meta_data = meta_data.replace("%{app_description}", app_description);
    meta_data = meta_data.replace("%{license}", license);
    meta_data = meta_data.replace("%{repository}", repository);
    meta_data = meta_data.replace("%{screenshots}", &screenshots);
    meta_data = meta_data.replace("%{releases}", releases_xml);

    let save_dir = assets_path.join("desktop");
    let save_path = save_dir.join(format!("{app_id}.metainfo.xml"));

    if !save_dir.is_dir() {
        fs::create_dir_all(&save_dir)?;
    }
    fs::write(&save_path, meta_data).inspect_err(|err| {
        error!(
            error = err.to_string(),
            path = &save_path.to_string_lossy().to_string(),
            "Failed to save metainfo"
        );
    })?;

    info!(
        metainfo_file = &save_path.to_string_lossy().to_string(),
        "Created new metainfo file:"
    );

    match Command::new("appstreamcli")
        .arg("validate")
        .arg("--no-net")
        .arg(save_path.as_os_str())
        .output()
    {
        Err(error) => {
            error!(error = error.to_string(), "Failed to validate metainfo");
            bail!(error)
        }
        Ok(output) => {
            if !output.status.success() {
                let error = utils::command::parse_output(&output.stdout);
                error!(error = error, "Failed to validate metainfo");
                bail!("Metainfo file does not validate!")
            }
        }
    }

    Ok(())
}

fn project_path() -> PathBuf {
    Path::new(".").canonicalize().unwrap()
}

fn assets_path() -> PathBuf {
    project_path().join("assets")
}

fn desktop_file_name() -> String {
    let app_id = config::APP_ID.get_value();
    let extension = "desktop";
    let file_name = format!("{app_id}.{extension}");

    file_name
}

fn icon_file_name() -> String {
    let app_id = config::APP_ID.get_value();
    let extension = "png";
    let file_name = format!("{app_id}.{extension}");

    file_name
}
