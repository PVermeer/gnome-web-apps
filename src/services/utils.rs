pub mod files {
    use anyhow::Result;
    use std::{
        fs::{self, DirEntry},
        path::Path,
    };

    pub fn get_entries_in_dir(dir: &Path) -> Result<Vec<DirEntry>> {
        fs::read_dir(dir)
            .into_iter()
            .flatten()
            .collect::<Result<Vec<_>, _>>()
            .map_err(std::convert::Into::into)
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

    pub fn is_flatpak_container() -> bool {
        env::var("container").is_ok_and(|value| value == "flatpak")
    }
}
