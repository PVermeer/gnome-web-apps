use crate::services::app_dirs::AppDirs;
use anyhow::Result;
use include_dir::{Dir, include_dir};
use std::{fs, rc::Rc};
use tracing::info;

static ASSETS: Dir = include_dir!("$CARGO_MANIFEST_DIR/assets");

pub struct Assets {
    app_dirs: Rc<AppDirs>,
}
impl Assets {
    pub fn new(data_dirs: &Rc<AppDirs>) -> Self {
        Self {
            app_dirs: data_dirs.clone(),
        }
    }

    pub fn init(&self) -> Result<()> {
        self.create_config_files()?;
        Ok(())
    }

    fn create_config_files(&self) -> Result<()> {
        let config_dir = self.app_dirs.config();

        // Docs: 'Fails if some files already exist' is not true.
        // It will overwrite existing files.
        info!("Creating / overwriting config files");
        ASSETS.extract(config_dir)?;

        Ok(())
    }

    pub fn reset_config_files(&self) -> Result<()> {
        let config_dir = self.app_dirs.config();

        if config_dir.is_dir() {
            info!("Deleting config files");
            fs::remove_dir_all(config_dir)?;
        }

        self.create_config_files()?;

        Ok(())
    }
}
