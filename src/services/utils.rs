pub mod files {
    use anyhow::{Context, Result};
    use std::{
        fs::{self, DirEntry},
        path::{Path, PathBuf},
    };

    pub fn get_entries_in_dir(dir: &Path) -> Result<Vec<DirEntry>> {
        fs::read_dir(dir)
            .into_iter()
            .flatten()
            .collect::<Result<Vec<_>, _>>()
            .map_err(std::convert::Into::into)
    }

    pub fn get_user_applications_dir() -> Result<PathBuf> {
        let applications_dir = std::env::home_dir()
            .context("Could not get user home dir")?
            .join(".local")
            .join("share")
            .join("applications");

        Ok(applications_dir)
    }
}
