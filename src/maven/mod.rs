pub mod coord;
pub mod fetch;
pub mod pom;
pub mod resolve;

use anyhow::Result;
use indicatif::MultiProgress;

use self::coord::MavenCoord;
use self::fetch::MavenFetcher;

/// Resolve Maven coordinates and return a colon-separated classpath of local JAR paths.
/// Progress is reported through the shared MultiProgress.
pub fn resolve_classpath(deps: &[String], mp: &MultiProgress) -> Result<String> {
    let coords: Vec<MavenCoord> = deps
        .iter()
        .map(|s| MavenCoord::parse(s))
        .collect::<Result<_>>()?;

    let fetcher = MavenFetcher::new(mp.clone())?;
    resolve::resolve_and_fetch(&fetcher, &coords)
}
