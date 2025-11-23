use anyhow::Result;
use include_dir::{Dir, include_dir};
use log::info;
use std::{
    cell::RefCell,
    fs,
    path::{Path, PathBuf},
    rc::Rc,
};

use crate::services::app_dirs::AppDirs;

static ASSETS: Dir = include_dir!("$CARGO_MANIFEST_DIR/assets");

pub struct Assets {
    app_dirs: Rc<AppDirs>,
    config_dir: RefCell<PathBuf>,
}
impl Assets {
    pub fn new(data_dirs: &Rc<AppDirs>) -> Self {
        Self {
            app_dirs: data_dirs.clone(),
            config_dir: RefCell::new(PathBuf::new()),
        }
    }

    pub fn init(&self) -> Result<()> {
        let mut config_dir = self.config_dir.borrow_mut();
        *config_dir = self.app_dirs.config();

        if cfg!(debug_assertions) {
            *config_dir = Path::new("dev-config").to_path_buf();
        }
        drop(config_dir);

        self.create_config_files()?;

        Ok(())
    }

    fn create_config_files(&self) -> Result<()> {
        let config_dir = &*self.config_dir.borrow();

        // Docs: 'Fails if some files already exist' is not true.
        // It will overwrite existing files.
        info!("Creating / overwriting config files");
        ASSETS.extract(config_dir)?;

        Ok(())
    }

    pub fn reset_config_files(&self) -> Result<()> {
        let config_dir = &*self.config_dir.borrow();

        if config_dir.is_dir() {
            info!("Deleting config files");
            fs::remove_dir_all(config_dir)?;
        }

        self.create_config_files()?;

        Ok(())
    }
}
