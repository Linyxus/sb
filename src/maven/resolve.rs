use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;

use anyhow::{bail, Result};

use super::coord::MavenCoord;
use super::fetch::MavenFetcher;
use super::pom::{self, ManagedDep, Pom};

struct QueueEntry {
    coord: MavenCoord,
    depth: u32,
    exclusions: HashSet<(String, String)>,
}

/// Resolve transitive dependencies for a list of root Maven coordinates.
/// Returns de-duplicated list of coordinates (compile+runtime scope only).
pub fn resolve(fetcher: &MavenFetcher, roots: &[MavenCoord]) -> Result<Vec<MavenCoord>> {
    let mut resolved: HashMap<(String, String), MavenCoord> = HashMap::new();
    let mut dep_mgmt: HashMap<(String, String), ManagedDep> = HashMap::new();
    let mut pom_cache: HashMap<MavenCoord, Pom> = HashMap::new();
    let mut queue: VecDeque<QueueEntry> = VecDeque::new();

    for coord in roots {
        queue.push_back(QueueEntry {
            coord: coord.clone(),
            depth: 0,
            exclusions: HashSet::new(),
        });
    }

    while let Some(entry) = queue.pop_front() {
        let key = entry.coord.key();

        if resolved.contains_key(&key) {
            continue;
        }

        resolved.insert(key, entry.coord.clone());

        let effective = match pom::resolve_effective_pom(fetcher, &entry.coord, &mut pom_cache) {
            Ok(p) => p,
            Err(e) => {
                if entry.depth > 0 {
                    eprintln!("warning: failed to resolve POM for {}: {e}", entry.coord);
                    continue;
                }
                return Err(e);
            }
        };

        for md in &effective.dependency_management {
            if md.dep_type == "pom" && md.scope.as_deref() == Some("import") {
                let bom_coord = MavenCoord::new(&md.group_id, &md.artifact_id, &md.version);
                if let Ok(bom_pom) = pom::resolve_effective_pom(fetcher, &bom_coord, &mut pom_cache) {
                    for bom_md in &bom_pom.dependency_management {
                        let bom_key = (bom_md.group_id.clone(), bom_md.artifact_id.clone());
                        dep_mgmt.entry(bom_key).or_insert_with(|| bom_md.clone());
                    }
                }
            } else {
                let md_key = (md.group_id.clone(), md.artifact_id.clone());
                dep_mgmt.entry(md_key).or_insert_with(|| md.clone());
            }
        }

        for dep in &effective.dependencies {
            let scope = dep.scope.as_str();
            if scope == "test" || scope == "provided" || scope == "system" {
                continue;
            }
            if dep.optional {
                continue;
            }

            let dep_key = (dep.group_id.clone(), dep.artifact_id.clone());

            if entry.exclusions.contains(&dep_key)
                || entry.exclusions.contains(&(dep.group_id.clone(), "*".to_string()))
            {
                continue;
            }

            if resolved.contains_key(&dep_key) {
                continue;
            }

            let version = if let Some(ref v) = dep.version {
                if v.is_empty() { None } else { Some(v.clone()) }
            } else {
                None
            };
            let version = version.or_else(|| {
                dep_mgmt.get(&dep_key).map(|md| md.version.clone())
            });

            let version = match version {
                Some(v) => v,
                None => {
                    eprintln!("warning: no version for {}:{}, skipping", dep.group_id, dep.artifact_id);
                    continue;
                }
            };

            if version.starts_with('[') || version.starts_with('(') {
                eprintln!("warning: version ranges not supported: {}:{}:{version}, skipping", dep.group_id, dep.artifact_id);
                continue;
            }

            let mut child_exclusions = entry.exclusions.clone();
            for excl in &dep.exclusions {
                child_exclusions.insert(excl.clone());
            }
            if let Some(md) = dep_mgmt.get(&dep_key) {
                for excl in &md.exclusions {
                    child_exclusions.insert(excl.clone());
                }
            }

            let child_coord = MavenCoord::new(&dep.group_id, &dep.artifact_id, &version);
            queue.push_back(QueueEntry {
                coord: child_coord,
                depth: entry.depth + 1,
                exclusions: child_exclusions,
            });
        }
    }

    Ok(resolved.into_values().collect())
}

/// Resolve dependencies and download all JARs in parallel. Returns classpath string.
pub fn resolve_and_fetch(fetcher: &MavenFetcher, roots: &[MavenCoord]) -> Result<String> {
    let coords = resolve(fetcher, roots)?;

    // Download JARs in parallel — each thread adds its own progress bar via fetcher
    let jar_paths: Vec<Result<PathBuf>> = std::thread::scope(|s| {
        let handles: Vec<_> = coords.iter().map(|c| {
            s.spawn(|| fetcher.fetch_jar(c))
        }).collect();
        handles.into_iter().map(|h| h.join().expect("JAR download panicked")).collect()
    });

    let mut paths = Vec::new();
    for result in jar_paths {
        match result {
            Ok(p) => paths.push(p),
            Err(e) => {
                let msg = format!("{e}");
                if msg.contains("HTTP 404") {
                    continue;
                }
                bail!("{e}");
            }
        }
    }

    let classpath = paths.iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join(":");

    Ok(classpath)
}
