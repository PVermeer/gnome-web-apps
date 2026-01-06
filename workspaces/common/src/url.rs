use anyhow::{Context, Result};
use url::Url;

pub trait UrlExt {
    fn base_url(&mut self) -> Result<String>;
    fn has_path(&self) -> bool;
}
impl UrlExt for Url {
    fn base_url(&mut self) -> Result<String> {
        self.path_segments_mut()
            .ok()
            .context("Failed to get path segments")?
            .clear();
        self.set_query(None);
        self.set_fragment(None);

        Ok(self.as_str().to_string())
    }

    fn has_path(&self) -> bool {
        self.path().len() > 1
    }
}
