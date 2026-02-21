use std::fs;
use std::io::Read;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};

use super::coord::MavenCoord;

pub struct MavenFetcher {
    cache_root: PathBuf,
    agent: ureq::Agent,
}

impl MavenFetcher {
    pub fn new() -> Result<Self> {
        let cache_root = dirs::cache_dir()
            .context("could not determine cache directory")?
            .join("sb")
            .join("maven");
        Ok(Self {
            cache_root,
            agent: ureq::Agent::new_with_defaults(),
        })
    }

    /// Fetch POM XML content. Returns cached content if available.
    pub fn fetch_pom(&self, coord: &MavenCoord) -> Result<String> {
        let local = coord.local_pom_path(&self.cache_root);
        if local.exists() {
            return fs::read_to_string(&local)
                .with_context(|| format!("failed to read cached POM: {}", local.display()));
        }
        let url = coord.pom_url();
        let body = self.http_get_string(&url)
            .with_context(|| format!("failed to fetch POM for {coord}"))?;
        if let Some(parent) = local.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&local, &body)?;
        Ok(body)
    }

    /// Download JAR to cache. Returns local path. Skips download if already cached.
    pub fn fetch_jar(&self, coord: &MavenCoord) -> Result<PathBuf> {
        let local = coord.local_jar_path(&self.cache_root);
        if local.exists() {
            return Ok(local);
        }
        let url = coord.jar_url();
        let bytes = self.http_get_bytes(&url)
            .with_context(|| format!("failed to fetch JAR for {coord}"))?;
        if let Some(parent) = local.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&local, &bytes)?;
        Ok(local)
    }

    fn http_get_string(&self, url: &str) -> Result<String> {
        let response = self.agent.get(url).call()
            .map_err(|e| anyhow::anyhow!("HTTP GET {url} failed: {e}"))?;
        let status = response.status();
        if status != 200 {
            bail!("HTTP {status} for {url}");
        }
        let mut body = String::new();
        response.into_body().as_reader().read_to_string(&mut body)?;
        Ok(body)
    }

    fn http_get_bytes(&self, url: &str) -> Result<Vec<u8>> {
        let response = self.agent.get(url).call()
            .map_err(|e| anyhow::anyhow!("HTTP GET {url} failed: {e}"))?;
        let status = response.status();
        if status != 200 {
            bail!("HTTP {status} for {url}");
        }
        let mut body = Vec::new();
        response.into_body().as_reader().read_to_end(&mut body)?;
        Ok(body)
    }
}
