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
