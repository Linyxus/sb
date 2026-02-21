use anyhow::Result;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::path::Path;
use std::time::Duration;

use crate::cache;
use crate::config::SbConfig;
use crate::maven;

#[derive(Debug, Clone)]
pub struct ResolvedClasspath {
    pub compiler_cp: String,
    pub user_cp: String,
}

impl ResolvedClasspath {
    fn serialize(&self) -> String {
        format!("{}\n{}", self.compiler_cp, self.user_cp)
    }

    fn deserialize(s: &str) -> Option<Self> {
        let mut lines = s.lines();
        let compiler_cp = lines.next()?.to_string();
        let user_cp = lines.next()?.to_string();
        Some(Self { compiler_cp, user_cp })
    }
}

pub fn dep_hash(config: &SbConfig) -> u64 {
    let mut items: Vec<String> = config.project.dependencies.clone();
    items.sort();
    items.insert(0, config.project.scala_version.clone());
    cache::hash_strings(&items)
}

fn spinner_style() -> ProgressStyle {
    ProgressStyle::with_template("{spinner:.cyan} {msg}")
        .unwrap()
        .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ ")
}

fn finish_style() -> ProgressStyle {
    ProgressStyle::with_template("{msg}").unwrap()
}

pub fn resolve_classpath(config: &SbConfig, project_root: &Path) -> Result<ResolvedClasspath> {
    let hash = dep_hash(config);
    let hash_str = hash.to_string();

    // Check cache
    if let Some(cached_hash) = cache::read_cache(project_root, "dep-hash") {
        if cached_hash == hash_str {
            if let Some(cached_cp) = cache::read_cache(project_root, "classpath") {
                if let Some(resolved) = ResolvedClasspath::deserialize(&cached_cp) {
                    return Ok(resolved);
                }
            }
        }
    }

    let sv = &config.project.scala_version;

    let mp = MultiProgress::new();

    let compiler_deps = vec![format!("org.scala-lang:scala3-compiler_3:{sv}")];
    let mut user_deps = vec![format!("org.scala-lang:scala3-library_3:{sv}")];
    for dep in &config.project.dependencies {
        user_deps.push(resolve_dep_coord(dep, sv));
    }

    let sp1 = mp.add(ProgressBar::new_spinner());
    sp1.set_style(spinner_style());
    sp1.set_message(format!("Resolving compiler {sv}"));
    sp1.enable_steady_tick(Duration::from_millis(80));

    let sp2 = mp.add(ProgressBar::new_spinner());
    sp2.set_style(spinner_style());
    sp2.set_message(format!("Resolving dependencies"));
    sp2.enable_steady_tick(Duration::from_millis(80));

    // Resolve both in parallel, passing progress bars for live updates
    let (compiler_result, user_result) = std::thread::scope(|s| {
        let h1 = s.spawn(|| maven::resolve_classpath(&compiler_deps, Some(&sp1), &format!("compiler {sv}")));
        let h2 = s.spawn(|| maven::resolve_classpath(&user_deps, Some(&sp2), "dependencies"));
        (
            h1.join().expect("compiler resolve panicked"),
            h2.join().expect("user resolve panicked"),
        )
    });

    let compiler_cp = compiler_result?;
    sp1.set_style(finish_style());
    sp1.finish_with_message(format!("✓ Resolved compiler {sv}"));

    let user_cp = user_result?;
    sp2.set_style(finish_style());
    let dep_count = config.project.dependencies.len();
    sp2.finish_with_message(format!(
        "✓ Resolved {} dependenc{}",
        dep_count,
        if dep_count == 1 { "y" } else { "ies" }
    ));

    let resolved = ResolvedClasspath { compiler_cp, user_cp };

    // Write cache
    cache::write_cache(project_root, "dep-hash", &hash_str)?;
    cache::write_cache(project_root, "classpath", &resolved.serialize())?;

    Ok(resolved)
}

/// Convert a user dependency string to a full Maven coordinate.
/// Handles `::` (Scala cross-version) by replacing with `_3:`.
fn resolve_dep_coord(dep: &str, _scala_version: &str) -> String {
    if let Some((org, rest)) = dep.split_once("::") {
        if let Some((name, version)) = rest.split_once(':') {
            return format!("{org}:{name}_3:{version}");
        }
    }
    dep.to_string()
}
