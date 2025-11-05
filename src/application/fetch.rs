use anyhow::{Result, bail};
use gtk::gio::{self};
use log::debug;
use log::error;
use std::time::Duration;
use ureq::Agent;

pub struct Fetch {
    agent: Agent,
}
impl Fetch {
    const FETCH_TIMEOUT: u64 = 5; // Seconds

    pub fn new() -> Self {
        let agent: Agent = Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(Self::FETCH_TIMEOUT)))
            .build()
            .into();

        Self { agent }
    }

    pub async fn get_as_string(&self, url: String) -> Result<String> {
        debug!("Fetching text from url: {url}");
        let agent_clone = self.agent.clone();
        let url = url.clone();
        let url_clone = url.clone();

        match gio::spawn_blocking(move || -> Result<String> {
            let response = agent_clone
                .get(url_clone)
                .call()?
                .body_mut()
                .read_to_string()?;
            Ok(response)
        })
        .await
        {
            Ok(Ok(text)) => Ok(text),
            Ok(Err(error)) => Self::error_handler(&url, &error),
            Err(error) => Self::error_handler(&url, &error),
        }
    }

    pub async fn get_as_bytes(&self, url: String) -> Result<Vec<u8>> {
        debug!("Fetching bytes from url: {url}");
        let agent_clone = self.agent.clone();
        let url = url.clone();
        let url_clone = url.clone();

        match gio::spawn_blocking(move || -> Result<Vec<u8>> {
            let response = agent_clone
                .get(url_clone)
                .call()?
                .body_mut()
                .read_to_vec()?;
            Ok(response)
        })
        .await
        {
            Ok(Ok(bytes)) => Ok(bytes),
            Ok(Err(error)) => Self::error_handler(&url, &error),
            Err(error) => Self::error_handler(&url, &error),
        }
    }

    // Any error logged and a anyhow::Error
    fn error_handler<R>(url: &str, error: impl std::fmt::Debug) -> Result<R> {
        let message = format!("Fetching '{url}' failed: '{error:?}'");
        error!("{message}");
        bail!(message)
    }
}
