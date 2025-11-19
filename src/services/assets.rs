use anyhow::{Context, Result};
use include_dir::{Dir, include_dir};
use log::info;
use std::{
    cell::RefCell,
    fs,
    path::{Path, PathBuf},
    rc::Rc,
};
use xdg::BaseDirectories;

static ASSETS: Dir = include_dir!("$CARGO_MANIFEST_DIR/assets");

pub struct Assets {
    app_dirs: Rc<BaseDirectories>,
    config_dir: RefCell<PathBuf>,
}
impl Assets {
    pub fn new(app_dirs: &Rc<BaseDirectories>) -> Self {
        Self {
            app_dirs: app_dirs.clone(),
            config_dir: RefCell::new(PathBuf::new()),
        }
    }

    pub fn init(&self) -> Result<()> {
        let mut config_dir = self.config_dir.borrow_mut();
        *config_dir = self
            .app_dirs
            .get_config_home()
            .context("Could not get user config dir")?;

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
