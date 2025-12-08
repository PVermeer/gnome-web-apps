pub mod files {
    use anyhow::{Context, Result, bail};
    use std::{
        fs::{self, DirEntry},
        os,
        path::Path,
    };
    use tracing::debug;

    pub fn get_entries_in_dir(dir: &Path) -> Result<Vec<DirEntry>> {
        fs::read_dir(dir)
            .into_iter()
            .flatten()
            .collect::<Result<Vec<_>, _>>()
            .map_err(std::convert::Into::into)
    }

    pub fn create_symlink(symlink_path: &Path, target: &Path) -> Result<()> {
        let mut target = target.to_path_buf();
        let a = symlink_path.display().to_string();
        let b = target.display().to_string();

        if !symlink_path.is_symlink() {
            let parent_path = symlink_path.parent().context(format!(
                "Could not get parent of dir: {}",
                symlink_path.display()
            ))?;

            if !parent_path.is_dir() && !parent_path.is_symlink() {
                fs::create_dir_all(parent_path).context(format!(
                    "Could not create parent dir: {}",
                    parent_path.display()
                ))?;
            }

            if !target.is_absolute() {
                if let Ok(target_absolute) = target.canonicalize() {
                    target = target_absolute;
                } else {
                    bail!(
                        "Could not create abosulte path for target: {}",
                        target.display()
                    );
                }
            }

            let result = os::unix::fs::symlink(&target, symlink_path).context(format!(
                "Could not create symlink: {} => {}",
                symlink_path.display(),
                target.display()
            ));
            if let Err(error) = result {
                debug!(symlink_path = a, target = b, "Failed to make a symlink");
                bail!(error)
            }

            debug!(symlink_path = a, target = b, "Made a symlink");
        }

        debug!(symlink_path = a, target = b, "Already a symlink");
        Ok(())
    }
}

pub mod env {
    use anyhow::Context;
    use std::{env, str::FromStr};
    use tracing::Level;

    pub fn get_log_level() -> Option<Level> {
        std::env::var("WAH_LOG")
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

pub mod log {
    use tracing::error;

    pub fn error(message: &str, error: Option<anyhow::Error>) {
        if let Some(error) = error {
            error!(message = message, "{error:?}");
        }
    }

    pub fn error_from_stderr(message: &str, output: &[u8]) {
        let error = String::from_utf8_lossy(output);
        error!(message = message, "{error:?}");
    }
}

pub mod command {
    use anyhow::{Result, bail};
    use gtk::glib;
    use std::{fmt::Write, process::Command};
    use tracing::debug;

    use crate::utils::{env, log};

    pub fn run_command_async(command: &str) -> Result<()> {
        let mut run_command = String::new();

        if env::is_flatpak_container() {
            write!(run_command, "flatpak-spawn --host")?;
            if env::is_devcontainer() {
                write!(run_command, " --env=DISPLAY=:0")?;
            }
        }
        write!(run_command, " {command}")?;

        debug!(command = run_command, "Running async command");
        glib::spawn_command_line_async(run_command).map_err(Into::into)
    }

    pub fn run_command_sync(command: &str) -> Result<String> {
        let mut run_command = String::new();

        if env::is_flatpak_container() {
            write!(run_command, "flatpak-spawn --host")?;
            if env::is_devcontainer() {
                write!(run_command, " --env=DISPLAY=:0")?;
            }
        }
        write!(run_command, " {command}")?;

        let mut args = glib::shell_parse_argv(&run_command)?;
        if args.is_empty() {
            bail!("Incorrect command")
        }
        let command = args.remove(0);

        debug!(command = run_command, "Running sync command");
        let output = Command::new(command).args(args).output()?;

        if !output.status.success() {
            let message = "Command failed";
            log::error_from_stderr("Command failed", &output.stderr);
            bail!(message)
        }
        let result = String::from_utf8_lossy(&output.stdout).to_string();

        Ok(result)
    }
}
