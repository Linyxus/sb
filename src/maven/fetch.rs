use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use super::coord::MavenCoord;

fn spinner_style() -> ProgressStyle {
    ProgressStyle::with_template("  {spinner:.cyan} {msg}")
        .unwrap()
        .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ ")
}

fn finish_style() -> ProgressStyle {
    ProgressStyle::with_template("  {msg}").unwrap()
}

struct TrackerState {
    completed: Vec<ProgressBar>,
    folded_count: u64,
}

/// Manages a rolling window of completed progress bars, folding older ones
/// into a summary line like "✓ N artifacts fetched".
pub struct ProgressTracker {
    mp: MultiProgress,
    summary_bar: ProgressBar,
    state: Mutex<TrackerState>,
}

const MAX_VISIBLE_COMPLETED: usize = 3;

impl ProgressTracker {
    pub fn new(mp: MultiProgress) -> Arc<Self> {
        let summary_bar = mp.add(ProgressBar::new_spinner());
        summary_bar.set_style(finish_style());
        summary_bar.finish_with_message(String::new());
        Arc::new(Self {
            mp,
            summary_bar,
            state: Mutex::new(TrackerState {
                completed: Vec::new(),
                folded_count: 0,
            }),
        })
    }

    pub fn add_spinner(&self, label: &str) -> ProgressBar {
        let pb = self.mp.add(ProgressBar::new_spinner());
        pb.set_style(spinner_style());
        pb.set_message(label.to_string());
        pb.enable_steady_tick(Duration::from_millis(80));
        pb
    }

    pub fn mark_done(&self, pb: &ProgressBar, label: &str) {
        let mut st = self.state.lock().unwrap();

        if st.completed.len() == MAX_VISIBLE_COMPLETED {
            let oldest = st.completed.remove(0);
            oldest.finish_and_clear();
            st.folded_count += 1;
            self.summary_bar.set_message(format!("✓ {} artifacts fetched", st.folded_count));
        }

        pb.set_style(finish_style());
        pb.finish_with_message(format!("✓ {label}"));
        st.completed.push(pb.clone());
    }
}

pub struct MavenFetcher {
    cache_root: PathBuf,
    agent: ureq::Agent,
    tracker: Arc<ProgressTracker>,
}

impl MavenFetcher {
    pub fn new(tracker: Arc<ProgressTracker>) -> Result<Self> {
        let cache_root = dirs::cache_dir()
            .context("could not determine cache directory")?
            .join("sb")
            .join("maven");
        Ok(Self {
            cache_root,
            agent: ureq::Agent::new_with_defaults(),
            tracker,
        })
    }

    /// Fetch POM XML content. Shows a per-artifact progress bar.
    pub fn fetch_pom(&self, coord: &MavenCoord) -> Result<String> {
        let label = format!("{}-{}.pom", coord.artifact_id, coord.version);
        let local = coord.local_pom_path(&self.cache_root);

        if local.exists() {
            let pb = self.tracker.add_spinner(&label);
            self.tracker.mark_done(&pb, &label);
            return fs::read_to_string(&local)
                .with_context(|| format!("failed to read cached POM: {}", local.display()));
        }

        let pb = self.tracker.add_spinner(&label);

        let url = coord.pom_url();
        let body = self.http_get_string(&url)
            .with_context(|| format!("failed to fetch POM for {coord}"))?;

        if let Some(parent) = local.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&local, &body)?;

        self.tracker.mark_done(&pb, &label);
        Ok(body)
    }

    /// Download JAR to cache. Shows a per-artifact progress bar.
    pub fn fetch_jar(&self, coord: &MavenCoord) -> Result<PathBuf> {
        let label = format!("{}-{}.jar", coord.artifact_id, coord.version);
        let local = coord.local_jar_path(&self.cache_root);

        if local.exists() {
            let pb = self.tracker.add_spinner(&label);
            self.tracker.mark_done(&pb, &label);
            return Ok(local);
        }

        let pb = self.tracker.add_spinner(&label);

        let url = coord.jar_url();
        let bytes = self.http_get_bytes(&url)
            .with_context(|| format!("failed to fetch JAR for {coord}"))?;

        if let Some(parent) = local.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&local, &bytes)?;

        self.tracker.mark_done(&pb, &label);
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
