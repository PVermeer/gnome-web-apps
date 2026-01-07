use anyhow::{Context, Result};
use url::Url;

pub trait UrlExt {
    fn get_base_url(&self) -> Result<Url>;
    fn has_path(&self) -> bool;
    fn sanitize(&self) -> Url;
}
impl UrlExt for Url {
    /// Get a new `URL` that only contains the base part
    fn get_base_url(&self) -> Result<Self> {
        let mut self_mut_clone = self.clone();

        self_mut_clone
            .path_segments_mut()
            .ok()
            .context("Failed to get path segments")?
            .clear();
        self_mut_clone.set_query(None);
        self_mut_clone.set_fragment(None);

        Ok(self_mut_clone)
    }

    fn has_path(&self) -> bool {
        self.path().len() > 1
    }

    /// Return a new `URL` with path or added '/' and without queries or fragments
    fn sanitize(&self) -> Self {
        let mut self_mut_clone = self.clone();

        self_mut_clone.set_query(None);
        self_mut_clone.set_fragment(None);
        if !self_mut_clone.path().ends_with('/') {
            self_mut_clone.set_path(&(self_mut_clone.path().to_owned() + "/"));
        }

        self_mut_clone
    }
}
