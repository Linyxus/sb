use anyhow::{bail, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::cache;
use crate::config::SbConfig;
use crate::resolve::{self, ResolvedClasspath};
use crate::tasty::deps::{self, IncrementalState};

pub struct CompileResult {
    pub resolved: ResolvedClasspath,
}

pub fn compile(config: &SbConfig, project_root: &Path) -> Result<CompileResult> {
    let src_dir = SbConfig::source_dir(project_root);
    let classes_dir = SbConfig::classes_dir(project_root);

    // Parallel: resolve deps + hash sources (per-file)
    let (resolved, src_result) = std::thread::scope(|s| {
        let resolve_handle = s.spawn(|| resolve::resolve_classpath(config, project_root));
        let hash_handle = s.spawn(|| cache::hash_sources_per_file(&src_dir, project_root));
        (
            resolve_handle.join().expect("resolve thread panicked"),
            hash_handle.join().expect("hash thread panicked"),
        )
    });
    let resolved = resolved?;
    let (new_hashes, sources) = src_result?;

    if sources.is_empty() {
        bail!("no .scala source files found in {}", src_dir.display());
    }

    let dep_hash_str = resolve::dep_hash(config).to_string();

    // Try incremental compilation
    if let Some(old_state) = IncrementalState::load(project_root) {
        if old_state.dep_hash == dep_hash_str && classes_dir.exists() {
            let (changed, added, deleted) =
                cache::diff_hashes(&old_state.source_hashes, &new_hashes);

            if changed.is_empty() && added.is_empty() && deleted.is_empty() {
                eprintln!("Nothing to compile.");
                return Ok(CompileResult { resolved });
            }

            return incremental_compile(
                config,
                project_root,
                &resolved,
                &classes_dir,
                &new_hashes,
                &dep_hash_str,
                &old_state,
                &changed,
                &added,
                &deleted,
                &sources,
            );
        }
    }

    // Also check the old aggregate hash for backward compat / first-time migration
    let (agg_hash, _) = cache::hash_sources(&src_dir)?;
    let agg_hash_str = agg_hash.to_string();
    let cached_src = cache::read_cache(project_root, "src-hash");
    let cached_dep = cache::read_cache(project_root, "dep-hash");
    if cached_src.as_deref() == Some(&agg_hash_str)
        && cached_dep.as_deref() == Some(&dep_hash_str)
        && classes_dir.exists()
        && std::fs::read_dir(&classes_dir)?.next().is_some()
    {
        eprintln!("Nothing to compile.");
        // Migrate: save incremental state for next time
        let tasty_files = deps::scan_classes_dir(&classes_dir, project_root)?;
        let state = IncrementalState {
            source_hashes: new_hashes,
            tasty_files,
            dep_hash: dep_hash_str,
        };
        state.save(project_root)?;
        return Ok(CompileResult { resolved });
    }

    full_compile(
        config,
        project_root,
        &resolved,
        &classes_dir,
        &sources,
        &new_hashes,
        &dep_hash_str,
    )
}

fn full_compile(
    config: &SbConfig,
    project_root: &Path,
    resolved: &ResolvedClasspath,
    classes_dir: &Path,
    sources: &[PathBuf],
    new_hashes: &std::collections::HashMap<String, u64>,
    dep_hash_str: &str,
) -> Result<CompileResult> {
    // Clean classes dir for fresh compile
    if classes_dir.exists() {
        std::fs::remove_dir_all(classes_dir)?;
    }
    std::fs::create_dir_all(classes_dir)?;

    eprintln!(
        "Compiling {} source file{}...",
        sources.len(),
        if sources.len() == 1 { "" } else { "s" }
    );

    invoke_dotc(config, resolved, classes_dir, sources, None)?;

    // Save incremental state
    let agg_hash = cache::hash_sources(&SbConfig::source_dir(project_root))?.0;
    cache::write_cache(project_root, "src-hash", &agg_hash.to_string())?;
    cache::write_cache(project_root, "dep-hash", dep_hash_str)?;

    let tasty_files = deps::scan_classes_dir(classes_dir, project_root)?;
    let state = IncrementalState {
        source_hashes: new_hashes.clone(),
        tasty_files,
        dep_hash: dep_hash_str.to_string(),
    };
    state.save(project_root)?;

    Ok(CompileResult {
        resolved: resolved.clone(),
    })
}

#[allow(clippy::too_many_arguments)]
fn incremental_compile(
    config: &SbConfig,
    project_root: &Path,
    resolved: &ResolvedClasspath,
    classes_dir: &Path,
    new_hashes: &std::collections::HashMap<String, u64>,
    dep_hash_str: &str,
    old_state: &IncrementalState,
    changed: &[String],
    added: &[String],
    deleted: &[String],
    all_sources: &[PathBuf],
) -> Result<CompileResult> {
    let rev_deps = old_state.reverse_dep_map();

    // Collect old API hashes for all source files
    let mut old_api_hashes: std::collections::HashMap<String, Option<u64>> =
        std::collections::HashMap::new();
    for src in old_state.source_hashes.keys() {
        old_api_hashes.insert(src.clone(), old_state.api_hash_for_source(src));
    }

    // Remove stale class/tasty files for deleted sources
    for del_src in deleted {
        remove_class_files_for_source(del_src, old_state, classes_dir);
    }

    // Fixed-point loop
    let mut to_recompile: HashSet<String> = HashSet::new();
    // Round 1: compile changed + added files only
    to_recompile.extend(changed.iter().cloned());
    to_recompile.extend(added.iter().cloned());

    // Also add dependents of deleted files
    for del_src in deleted {
        if let Some(dependents) = rev_deps.get(del_src) {
            to_recompile.extend(dependents.iter().cloned());
        }
    }

    let mut round = 0;
    loop {
        round += 1;
        if to_recompile.is_empty() {
            break;
        }

        // Remove stale class/tasty files for files we're about to recompile
        for src in &to_recompile {
            remove_class_files_for_source(src, old_state, classes_dir);
        }

        // Resolve source paths
        let compile_sources: Vec<PathBuf> = all_sources
            .iter()
            .filter(|s| {
                let rel = s
                    .strip_prefix(project_root)
                    .unwrap_or(s)
                    .to_string_lossy()
                    .to_string();
                to_recompile.contains(&rel)
            })
            .cloned()
            .collect();

        if compile_sources.is_empty() {
            break;
        }

        if round == 1 {
            eprintln!(
                "Compiling {} source file{}...",
                compile_sources.len(),
                if compile_sources.len() == 1 { "" } else { "s" }
            );
        } else {
            eprintln!(
                "Compiling {} source file{} (round {})...",
                compile_sources.len(),
                if compile_sources.len() == 1 { "" } else { "s" },
                round,
            );
        }

        // Invoke dotc with classes_dir on classpath so compiler sees unchanged files
        invoke_dotc(config, resolved, classes_dir, &compile_sources, Some(classes_dir))?;

        // Re-scan tasty files to get new API hashes
        let new_tasty_files = deps::scan_classes_dir(classes_dir, project_root)?;

        // Build new state temporarily to check API changes
        let tmp_state = IncrementalState {
            source_hashes: new_hashes.clone(),
            tasty_files: new_tasty_files,
            dep_hash: dep_hash_str.to_string(),
        };

        // Check which recompiled files had API changes
        let new_rev_deps = tmp_state.reverse_dep_map();
        let mut next_round: HashSet<String> = HashSet::new();

        for src in &to_recompile {
            let old_api = old_api_hashes.get(src).copied().flatten();
            let new_api = tmp_state.api_hash_for_source(src);
            if old_api != new_api {
                // API changed — add dependents to next round
                if let Some(dependents) = new_rev_deps.get(src) {
                    for dep in dependents {
                        // Don't re-add files we just compiled in this round
                        if !to_recompile.contains(dep) {
                            next_round.insert(dep.clone());
                        }
                    }
                }
                // Also check old rev deps
                if let Some(dependents) = rev_deps.get(src) {
                    for dep in dependents {
                        if !to_recompile.contains(dep) {
                            next_round.insert(dep.clone());
                        }
                    }
                }
            }
            // Update old_api_hashes for next round comparison
            old_api_hashes.insert(src.clone(), new_api);
        }

        // Save state after each round (so we have latest tasty info)
        tmp_state.save(project_root)?;

        to_recompile = next_round;

        // Safety: prevent infinite loops
        if round > 100 {
            eprintln!("warning: incremental compilation exceeded 100 rounds, stopping");
            break;
        }
    }

    // Save final aggregate hash for backward compat
    let agg_hash = cache::hash_sources(&SbConfig::source_dir(project_root))?.0;
    cache::write_cache(project_root, "src-hash", &agg_hash.to_string())?;
    cache::write_cache(project_root, "dep-hash", dep_hash_str)?;

    Ok(CompileResult {
        resolved: resolved.clone(),
    })
}

fn remove_class_files_for_source(
    source_rel: &str,
    old_state: &IncrementalState,
    classes_dir: &Path,
) {
    // Find tasty files from this source
    for info in &old_state.tasty_files {
        if info.source_file == source_rel {
            let stem = info.tasty_path.trim_end_matches(".tasty");
            // Remove .tasty, .class, and inner class files (Foo$.class, etc.)
            let tasty = classes_dir.join(&info.tasty_path);
            let _ = std::fs::remove_file(&tasty);
            let class = classes_dir.join(format!("{stem}.class"));
            let _ = std::fs::remove_file(&class);
            let dollar_class = classes_dir.join(format!("{stem}$.class"));
            let _ = std::fs::remove_file(&dollar_class);

            // Also try to remove common inner class patterns
            if let Some(parent) = tasty.parent() {
                if let Ok(entries) = std::fs::read_dir(parent) {
                    let prefix = Path::new(stem)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    for entry in entries.flatten() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        if name.starts_with(&format!("{prefix}$")) && name.ends_with(".class") {
                            let _ = std::fs::remove_file(entry.path());
                        }
                    }
                }
            }
        }
    }
}

fn invoke_dotc(
    config: &SbConfig,
    resolved: &ResolvedClasspath,
    classes_dir: &Path,
    sources: &[PathBuf],
    extra_cp_dir: Option<&Path>,
) -> Result<()> {
    let mut cmd = Command::new("java");
    cmd.arg("--sun-misc-unsafe-memory-access=allow");
    cmd.arg("-cp").arg(&resolved.compiler_cp);
    cmd.arg("dotty.tools.dotc.Main");

    // Build classpath: optionally prepend classes_dir for incremental
    let cp = if let Some(extra) = extra_cp_dir {
        format!("{}:{}", extra.display(), resolved.user_cp)
    } else {
        resolved.user_cp.clone()
    };
    cmd.arg("-classpath").arg(&cp);

    cmd.arg("-d").arg(classes_dir);
    for opt in &config.project.scalac_options {
        cmd.arg(opt);
    }
    for src in sources {
        cmd.arg(src);
    }

    let status = cmd.status()?;
    if !status.success() {
        bail!("compilation failed");
    }
    Ok(())
}
