pub mod files {
    use anyhow::{Context, Result, bail};
    use std::{
        fs::{self, DirEntry},
        os,
        path::{self, Path},
    };
    use tracing::{debug, error};

    pub fn get_entries_in_dir(dir: &Path) -> Result<Vec<DirEntry>> {
        fs::read_dir(dir)
            .into_iter()
            .flatten()
            .collect::<Result<Vec<_>, _>>()
            .map_err(std::convert::Into::into)
    }

    pub fn create_symlink(symlink_path: &Path, target: &Path) {
        if let Err(error) = (|| -> Result<()> {
            let mut target = target.to_path_buf();

            if !symlink_path.is_symlink() {
                let parent_path = symlink_path.parent().context(format!(
                    "Could not get parent of dir: {}",
                    symlink_path.display()
                ))?;

                if !parent_path.is_dir() {
                    fs::create_dir_all(parent_path).context(format!(
                        "Could not create parent dir: {}",
                        symlink_path.display()
                    ))?;
                }

                if !target.is_absolute() {
                    if let Ok(target_absolute) = path::absolute(&target) {
                        target = target_absolute;
                    } else {
                        bail!(
                            "Could not create abosulte path for target: {}",
                            target.display()
                        );
                    }
                }

                os::unix::fs::symlink(&target, symlink_path).context(format!(
                    "Could not create symlink: {} => {}",
                    symlink_path.display(),
                    target.display()
                ))?;
            }

            let a = symlink_path.display().to_string();
            let b = target.display().to_string();
            debug!(symlink_path = a, target = b, "Made a symlink");

            Ok(())
        })() {
            error!("Error creating symlink: {error:?}");
        }
    }
}

pub mod env {
    use std::{env, str::FromStr};

    use anyhow::Context;
    use tracing::Level;

    pub fn get_log_level() -> Option<Level> {
        std::env::var("RUST_LOG")
            .with_context(|| {
                let info = "No LOG environment variable set";
                println!("{info}");
                info
            })
            .and_then(|level_str| {
                Level::from_str(&level_str).with_context(|| {
                    let error =
                        format!("Invalid LOG environment variable set, using '{level_str}'");
                    eprintln!("{error:?}");
                    error
                })
            })
            .ok()
    }

    pub fn is_devcontainer() -> bool {
        env::var("RUN_IN_VSCODE_DEVCONTAINER").is_ok()
    }

    pub fn is_flatpak_container() -> bool {
        env::var("container").is_ok_and(|value| value == "flatpak")
    }
}

pub mod strings {
    pub fn capitalize(string: &str) -> String {
        let mut chars = string.chars();
        chars
            .next()
            .unwrap_or_default()
            .to_uppercase()
            .collect::<String>()
            + chars.as_str()
    }

    pub fn capitalize_all_words(string: &str) -> String {
        string
            .split(' ')
            .map(capitalize)
            .collect::<Vec<_>>()
            .join(" ")
    }
}
