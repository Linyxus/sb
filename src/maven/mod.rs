pub mod coord;
pub mod fetch;
pub mod pom;
pub mod resolve;

use std::sync::Arc;

use anyhow::Result;

use self::coord::MavenCoord;
use self::fetch::{MavenFetcher, ProgressTracker};

/// Resolve Maven coordinates and return a colon-separated classpath of local JAR paths.
pub fn resolve_classpath(deps: &[String], tracker: &Arc<ProgressTracker>) -> Result<String> {
    let coords: Vec<MavenCoord> = deps
        .iter()
        .map(|s| MavenCoord::parse(s))
        .collect::<Result<_>>()?;

    let fetcher = MavenFetcher::new(Arc::clone(tracker))?;
    resolve::resolve_and_fetch(&fetcher, &coords)
}
