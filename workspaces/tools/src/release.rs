use anyhow::{Context, Result, bail};
use chrono::DateTime;
use clap::Parser;
use common::{
    assets,
    config::{self, OnceLockExt},
    utils::{self, command},
};
use freedesktop_desktop_entry::DesktopEntry;
use git_cliff::args::Opt;
use regex::Regex;
use semver::Version;
use std::{fmt::Write as _, io::Write, process::Stdio, sync::OnceLock};
use std::{
    fs::{self, File},
    path::{Path, PathBuf},
    process::Command,
};
use tracing::{Level, error, info};
use tracing_subscriber::{FmtSubscriber, util::SubscriberInitExt};

static FLATPAK_MANIFEST_IN: &str = include_str!("../../../flatpak/manifest.in");
static CARGO_TOML: &str = include_str!("../../../workspaces/app/Cargo.toml");
static DRY_RUN: OnceLock<bool> = OnceLock::new();

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Perform a dry run without making any git changes
    #[arg(long)]
    dry_run: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    DRY_RUN.set(args.dry_run).unwrap_or_default();
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

    if *DRY_RUN.get_value() {
        info!("Running in dry-run mode");
    }

    dependency_check()?;
    config::init();
    config::log_all_values_debug();

    update_submodules()?;
    create_app_desktop_file()?;
    create_app_icon()?;

    let (releases_xml, new_version) = generate_changelog()?;
    update_cargo_with_new_version(&new_version)?;
    update_flatpak_manifest(&new_version)?;
    create_app_metainfo_file(&releases_xml, &new_version)?;
    generate_cargo_sources()?;
    create_release_in_git(&new_version)?;
    validate_metainfo(false)?;
    build_release_flatpak()?;
    create_flathub_release_pr(&new_version)?;

    info!("==== Finished release version {new_version}");

    Ok(())
}

fn dependency_check() -> Result<()> {
    let dependencies = [
        "git",
        "python3",
        "pipx",
        "flatpak-builder",
        "appstreamcli",
        "gh",
    ];
    let mut missing_dependencies = Vec::new();

    for dep in dependencies {
        let has_dependency = command::test_command_available_sync(dep);
        if !has_dependency {
            missing_dependencies.push(dep);
        }
    }

    let output = command::run_command_sync("gh auth status")?;
    println!("{}", output.stderr);
    println!("{}", output.stdout);

    if std::env::var("FLATHUB_TOKEN").is_err()
        && command::run_command_sync("gh auth status").is_err()
    {
        missing_dependencies.push(
            "Not logged in to github (gh command) or FLATHUB_TOKEN environment variable not defined",
        );
    }

    if missing_dependencies.is_empty() {
        return Ok(());
    }

    error!("Please provide the following dependencies:");
    for missing_dep in missing_dependencies {
        println!("{missing_dep}");
    }

    bail!("Missing some dependencies")
}

fn create_app_desktop_file() -> Result<()> {
    info!("==== Creating app desktop file");

    let desktop_file = assets::get_desktop_file_in();
    let app_id = config::APP_ID.get_value();
    let app_name = config::APP_NAME.get_value();
    let bin_name = config::BIN_NAME.get_value();
    let file_name = desktop_file_name();
    let save_path = assets_desktop_path().join(file_name);

    let mut base_desktop_file =
        DesktopEntry::from_str(&save_path, desktop_file, None::<&[String]>)?;

    base_desktop_file.add_desktop_entry("Name".to_string(), app_name.clone());
    base_desktop_file.add_desktop_entry("Icon".to_string(), app_id.clone());
    base_desktop_file.add_desktop_entry("StartupWMClass".to_string(), app_id.clone());
    base_desktop_file.add_desktop_entry("Exec".to_string(), bin_name.clone());

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
    let save_path = assets_desktop_path().join(file_name);

    let mut icon_file = File::create(&save_path)?;
    icon_file
        .write_all(assets::get_icon_data_in())
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

#[allow(clippy::too_many_lines)] // No exports of types from git_cliff...
fn generate_changelog() -> Result<(String, Version)> {
    info!("==== Generating changelogs");

    let changelog_path = &project_path().join("CHANGELOG.md");
    let mut changelog_file = &File::create(changelog_path)?;
    let mut git_cliff_args = Opt::parse_from([""]);
    git_cliff_args.config = project_path()
        .join("workspaces")
        .join("tools")
        .join("git-cliff.toml");
    let mut changelog = git_cliff::run(git_cliff_args.clone())?;

    let Ok(last_released_version) = changelog
        .releases
        .last()
        .and_then(|release| {
            release
                .version
                .clone()
                .map(|version| Version::parse(&version[1..]))
        })
        .unwrap_or(Ok(Version::new(0, 0, 0)))
    else {
        bail!("Could not determine last released version from git");
    };

    let Some(Ok(new_release_version)) = changelog
        .bump_version()
        .inspect_err(|err| {
            error!(
                error = err.to_string(),
                "Failed to create a new semantic version"
            );
        })?
        .map(|version| Version::parse(&version[1..]))
    else {
        bail!("Failed to create a new semantic version, no new changes?")
    };

    info!(
        last_released_version = last_released_version.to_string(),
        new_release_version = new_release_version.to_string()
    );

    // Remove initial release
    changelog.releases.pop();

    let last_n_releases = if changelog.releases.len() < 5 {
        changelog.releases.len()
    } else {
        5
    };
    let _ = changelog.releases.split_off(last_n_releases);

    changelog.generate(&mut changelog_file).inspect_err(|err| {
        error!(error = err.to_string(), "Failed to generate changelog");
    })?;

    info!(
        changelog = changelog_path.to_string_lossy().to_string(),
        "Written new changelog:"
    );

    // === Start of metainfo.xml parsing

    let mut all_releases_xml = String::new();

    for release in changelog.releases {
        let Some(Ok(version)) = release.version.map(|version| Version::parse(&version[1..])) else {
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

    Ok((all_releases_xml, new_release_version))
}

fn update_cargo_with_new_version(new_version: &Version) -> Result<()> {
    info!("==== Updating to new version");

    let version_re = Regex::new(r#"(?m)^version = "[0-9]+\.[0-9]+\.[0-9]+"$"#)?;
    let replacement = format!(r#"version = "{new_version}""#);
    let new_cargo_toml = version_re.replace(CARGO_TOML, replacement).to_string();
    let cargo_toml_file_path = &cargo_toml_file();

    fs::write(cargo_toml_file_path, new_cargo_toml).inspect_err(|err| {
        error!(
            error = err.to_string(),
            path = &cargo_toml_file_path.to_string_lossy().to_string(),
            "Failed to save updated cargo.toml"
        );
    })?;

    info!("Updating lockfile with new version");
    let command = "cargo";
    let args = ["generate-lockfile", "--offline"];
    match Command::new(command)
        .args(args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
    {
        Err(error) => {
            error!(error = error.to_string(), "Failed to run cargo");
            bail!(error)
        }
        Ok(output) => {
            if !output.status.success() {
                let error = utils::command::parse_output(&output.stderr);
                error!(error = error, "Failed to update lockfile");
                bail!("Failed to update lockfile")
            }
        }
    }

    info!("Updated cargo with new version");
    Ok(())
}

fn update_flatpak_manifest(new_version: &Version) -> Result<()> {
    info!("==== Updating flatpak manifest");

    let app_id = config::APP_ID.get_value();
    let app_name = config::APP_NAME.get_value();
    let app_name_dense = config::APP_NAME_DENSE.get_value();
    let app_name_short = config::APP_NAME_SHORT.get_value();
    let app_name_hyphen = config::APP_NAME_HYPHEN.get_value();
    let bin_name = config::BIN_NAME.get_value();
    let git_repository = &format!("{}.git", config::REPOSITORY.get_value());
    let git_tag = &format!("v{new_version}");

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

    let save_path_dev = &flatpak_dev_manifest();

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

    let save_path = &flatpak_release_manifest();

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

fn create_app_metainfo_file(releases_xml: &str, new_version: &Version) -> Result<()> {
    info!("==== Creating metainfo.xml");

    let app_id = config::APP_ID.get_value();
    let app_name = config::APP_NAME.get_value();
    let developer = config::DEVELOPER.get_value();
    let developer_id = &developer.to_lowercase();
    let app_summary = config::APP_SUMMARY.get_value();
    let app_description = config::APP_DESCRIPTION.get_value();
    let license = config::LICENSE.get_value();
    let repository = config::REPOSITORY.get_value();
    let git_tag = &format!("v{new_version}");

    let mut repository_split = repository.split('/');
    let repository_name = repository_split
        .next_back()
        .context("Failed split of repository name")?;
    let repository_org = repository_split
        .next_back()
        .context("Failed split of repository org")?;

    let screenshot_base_url = &format!(
        "https://raw.githubusercontent.com/{repository_org}/{repository_name}/refs/tags/{git_tag}/assets/screenshots"
    );
    let mut i = 0;
    let mut screenshots_files = utils::files::get_entries_in_dir(&assets_screenshots_path())?;
    screenshots_files.sort_by_key(|entry| {
        entry
            .file_name()
            .to_string_lossy()
            .split('-')
            .next()
            .and_then(|n| n.parse::<u32>().ok())
            .unwrap_or(0)
    });
    let screenshots = screenshots_files
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

    let mut meta_data = assets::get_meta_info_in().to_string();
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
    meta_data = meta_data.replace("%{git_tag}", git_tag);

    let save_path = flatpak_metainfo_xml();

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

    validate_metainfo(true)?;

    Ok(())
}

fn update_submodules() -> Result<()> {
    info!("==== Updating submodules");

    let work_dir = &project_path();
    let shell_script = r"
        set -e
        git submodule update --recursive --remote
    ";
    let error_message = "Failed to update submodules";
    run_shell_script(shell_script, work_dir, error_message)?;

    Ok(())
}

fn generate_cargo_sources() -> Result<()> {
    info!("==== Generating cargo sources");

    let sub_module_dir = &project_path()
        .join("external")
        .join("flatpak-builder-tools");
    let work_dir = &sub_module_dir.join("cargo");
    let project_root_from_work_dir = &Path::new(work_dir)
        .join("..")
        .join("..")
        .join("..")
        .canonicalize()?;
    let cargo_lock_path = &Path::new(project_root_from_work_dir)
        .join("Cargo.lock")
        .to_string_lossy()
        .to_string();
    let cargo_sources_path = &Path::new(project_root_from_work_dir)
        .join("flatpak")
        .join("cargo-sources.json")
        .to_string_lossy()
        .to_string();

    let shell_script = &format!(
        r#"
        set -e

        echo -e "\n==Installing poetry packages\n"
        pipx install poetry
        poetry install

        echo -e "\n==Running flatpak-cargo-generator.py\n"
        poetry run python3 flatpak-cargo-generator.py "{cargo_lock_path}" -o "{cargo_sources_path}"

        echo "== Done"
    "#
    );

    let command = "sh";
    let args = &["-c", shell_script];
    let error_message = "Failed to run flatpak-cargo-generator";
    match Command::new(command)
        .current_dir(work_dir)
        .args(args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
    {
        Err(error) => {
            error!(
                command = command,
                work_dir = work_dir.to_string_lossy().to_string(),
                error = %error.to_string(),
                "Failed to run command"
            );
            bail!(error)
        }
        Ok(output) => {
            if !output.status.success() {
                let error = utils::command::parse_output(&output.stderr);
                error!(
                    command = command,
                    args = %args.join(" "),
                    error = %error,
                    error_message,
                );
                bail!(error_message)
            }
        }
    }

    info!(
        cargo_sources_file = &cargo_sources_path,
        "Created cargo sources file:"
    );

    Ok(())
}

fn create_release_in_git(new_version: &Version) -> Result<()> {
    info!("==== Creating release in git");

    let version = format!("v{new_version}");

    let shell_script = &format!(
        r#"
        set -e
        git --no-pager diff --compact-summary --color=always
        echo ""
        git commit -a -m "chore(release): {version}" || true
        git tag -a {version} -m "Release version {new_version}"
        git push --follow-tags
    "#
    );

    let command = "sh";
    let args = &["-c", shell_script];
    let error_message = "Failed to create release in git";

    if *DRY_RUN.get_value() {
        println!("Dry-run - Would have run:\n{shell_script}");
        return Ok(());
    }

    match Command::new(command)
        .args(args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
    {
        Err(error) => {
            error!(command = command, error = %error.to_string(), error_message);
            bail!(error)
        }
        Ok(output) => {
            if !output.status.success() {
                let error = utils::command::parse_output(&output.stderr);
                error!(
                    command = command,
                    args = %args.join(" "),
                    error = %error,
                    error_message,
                );
                bail!(error_message)
            }
        }
    }

    Ok(())
}

fn build_release_flatpak() -> Result<()> {
    info!("==== Building flatpak");

    let flatpak_release_manifest_file = &flatpak_release_manifest().to_string_lossy().to_string();
    let target_dir = &project_path()
        .join("target")
        .join("flatpak-release")
        .to_string_lossy()
        .to_string();

    let command = "flatpak-builder";
    let args = [
        "--install-deps-from=flathub",
        &format!("--repo={target_dir}/repo"),
        &format!("--state-dir={target_dir}/.flatpak-builder"),
        "--force-clean",
        "--install",
        "--user",
        "--disable-rofiles-fuse",
        "--disable-cache",
        "--mirror-screenshots-url=https://dl.flathub.org/media/",
        &format!("{target_dir}/build"),
        flatpak_release_manifest_file,
    ];

    if *DRY_RUN.get_value() {
        println!("Dry-run - Would have run: flatpak builder");
        return Ok(());
    }

    match Command::new(command)
        .args(args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
    {
        Err(error) => {
            error!(error = error.to_string(), "Failed to run flatpak-builder");
            bail!(error)
        }
        Ok(output) => {
            if !output.status.success() {
                let error = utils::command::parse_output(&output.stderr);
                error!(error = error, "Failed to build release flatpak");
                bail!("Failed to build release flatpak")
            }
        }
    }

    info!("Successfully created a flatpak release package");

    Ok(())
}

fn validate_metainfo(offline: bool) -> Result<()> {
    info!("==== Validating metainfo.xml (online: {})", !offline);
    let mut offline = offline;

    if *DRY_RUN.get_value() && !offline {
        println!(
            "Dry-run - Would have run validate metainfo with online checks, now running offline"
        );
        offline = true;
    }

    let mut command = Command::new("appstreamcli");
    command.arg("validate");
    if offline {
        command.arg("--no-net");
    }
    command.arg(flatpak_metainfo_xml());

    match command.stdout(Stdio::inherit()).output() {
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
            Ok(())
        }
    }
}

#[allow(clippy::too_many_lines)] // You're perfect the way you are :)
fn create_flathub_release_pr(new_version: &Version) -> Result<()> {
    info!("==== Creating flathub release pr");

    let flathub_repo_dir = &flathub_repo();
    let app_id = config::APP_ID.get_value();
    let pr_branch = format!("v{new_version}");

    // Make changes on a new branch
    let shell_script = &format!(
        r#"
        set -e
        git checkout -B {pr_branch}
        echo ""
    "#
    );
    let error_message = "Failed to create new branch on flathub repo";
    run_shell_script(shell_script, flathub_repo_dir, error_message)?;

    let flatpak_release_manifest = &flatpak_release_manifest();
    let flatpak_release_manifest_flathub = flathub_repo_dir.join(
        flatpak_release_manifest
            .file_name()
            .context("No filename on flatpak manifest???")?,
    );
    let cargo_sources = flatpak_cargo_sources();
    let cargo_sources_flathub = flathub_repo_dir.join(
        cargo_sources
            .file_name()
            .context("No filename on cargo sources???")?,
    );

    fs::copy(flatpak_release_manifest, flatpak_release_manifest_flathub)?;
    fs::copy(cargo_sources, cargo_sources_flathub)?;

    let flathub_token = std::env::var("FLATHUB_TOKEN").unwrap_or_default();
    let mut git_remote = String::from("origin");
    if is_github_ssh_connected() {
        git_remote = format!("git@github.com:flathub/{app_id}");
        println!("Using SSH");
    } else {
        println!("Using https");
    }

    // Commit changes
    let shell_script = &format!(
        r#"
        set -e
        git commit -a -m "chore(automated-release): {pr_branch}" || true 
        git push {git_remote} {pr_branch} --force
        git fetch
        echo ""
    "#
    );
    let error_message = "Failed to push new branch on flathub repo";
    run_shell_script(shell_script, flathub_repo_dir, error_message)?;

    // Create the PR
    let pr_title = &format!(r"--title={pr_branch}");
    let pr_body = &format!(r"--body=Automatic release for {new_version}");
    let command = "gh";
    let mut args = ["pr", "create", pr_title, pr_body, "--draft"].to_vec();
    let error_message = "Failed to create a new PR on flathub repo";

    if *DRY_RUN.get_value() {
        println!("Dry-run - Adding --dry-run to github PR command");
        args.push("--dry-run");
    }
    match Command::new(command)
        .args(&args)
        .current_dir(flathub_repo_dir)
        .env("GH_TOKEN", flathub_token)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
    {
        Err(error) => {
            error!(command = command, error = %error.to_string(), error_message);
            bail!(error)
        }
        Ok(output) => {
            if !output.status.success() {
                let error = utils::command::parse_output(&output.stderr);
                error!(
                    command = command,
                    args = %args.join(" "),
                    error = %error,
                    error_message,
                );
                bail!(error_message.to_string())
            }
        }
    }

    // Revert flathub repo back to master and some cleanup
    update_submodules()?;

    if *DRY_RUN.get_value() {
        let shell_script = &format!(
            r"
            git push -d -f {git_remote} {pr_branch}
            git branch -d -f {pr_branch}
        "
        );
        let error_message = &format!(
            "Dry run - Failed to remove remote branch on {}",
            flathub_repo_dir.to_string_lossy()
        );
        run_shell_script(shell_script, flathub_repo_dir, error_message)?;
    }

    let shell_script = r"
            git fetch --prune
        ";
    let error_message = &format!(
        "Failed to prune branches on {}",
        flathub_repo_dir.to_string_lossy()
    );
    run_shell_script(shell_script, flathub_repo_dir, error_message)?;

    info!("Created new release PR in flathub repo");

    Ok(())
}

fn project_path() -> PathBuf {
    Path::new(".").canonicalize().unwrap()
}

fn assets_path() -> PathBuf {
    let path = project_path().join("assets");
    if !path.is_dir() {
        fs::create_dir_all(&path).unwrap();
    }
    path
}

fn assets_desktop_path() -> PathBuf {
    let path = assets_path().join("desktop");
    if !path.is_dir() {
        fs::create_dir_all(&path).unwrap();
    }
    path
}

fn assets_screenshots_path() -> PathBuf {
    let path = assets_path().join("screenshots");
    if !path.is_dir() {
        fs::create_dir_all(&path).unwrap();
    }
    path
}

fn flatpak_path() -> PathBuf {
    let path = project_path().join("flatpak");
    if !path.is_dir() {
        fs::create_dir_all(&path).unwrap();
    }
    path
}

fn cargo_toml_file() -> PathBuf {
    project_path()
        .join("workspaces")
        .join("app")
        .join("Cargo.toml")
}

fn flatpak_release_manifest() -> PathBuf {
    let app_id = config::APP_ID.get_value();
    let flatpak_release_manifest_name = &format!("{app_id}.yml");
    flatpak_path().join(flatpak_release_manifest_name)
}

fn flatpak_dev_manifest() -> PathBuf {
    let app_id = config::APP_ID.get_value();
    let flatpak_dev_manifest_name = &format!("{app_id}.Devel.yml");
    flatpak_path().join(flatpak_dev_manifest_name)
}

fn flatpak_metainfo_xml() -> PathBuf {
    let app_id = config::APP_ID.get_value();
    assets_desktop_path().join(format!("{app_id}.metainfo.xml"))
}

fn flatpak_cargo_sources() -> PathBuf {
    flatpak_path().join("cargo-sources.json")
}

fn flathub_repo() -> PathBuf {
    let app_id = config::APP_ID.get_value();
    project_path().join("external").join(app_id)
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

fn is_github_ssh_connected() -> bool {
    command::run_command_sync("ssh -T git@github.com")
        .map(|response| response.status == 1)
        .unwrap_or(false)
}

fn run_shell_script(shell_script: &str, work_dir: &Path, error_message: &str) -> Result<()> {
    let command = "sh";
    let args = &["-c", shell_script];

    match Command::new(command)
        .args(args)
        .current_dir(work_dir)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
    {
        Err(error) => {
            error!(command = command, error = %error.to_string(), error_message);
            bail!(error)
        }
        Ok(output) => {
            if !output.status.success() {
                let error = utils::command::parse_output(&output.stderr);
                error!(
                    command = command,
                    args = %args.join(" "),
                    error = %error,
                    error_message,
                );
                bail!(error_message.to_string())
            }
            Ok(())
        }
    }
}
