use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use xxhash_rust::xxh3::Xxh3;

pub fn hash_strings(items: &[String]) -> u64 {
    let mut hasher = Xxh3::new();
    for item in items {
        hasher.update(item.as_bytes());
        hasher.update(b"\0");
    }
    hasher.digest()
}

/// Hash all .scala files under `src_dir`, returning (hash, sorted file list).
pub fn hash_sources(src_dir: &Path) -> Result<(u64, Vec<PathBuf>)> {
    let mut files: Vec<PathBuf> = WalkDir::new(src_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file()
                && e.path().extension().is_some_and(|ext| ext == "scala")
        })
        .map(|e| e.into_path())
        .collect();
    files.sort();

    let mut hasher = Xxh3::new();
    for file in &files {
        let content = std::fs::read(file)?;
        // Include path in hash so renames are detected
        hasher.update(file.to_string_lossy().as_bytes());
        hasher.update(b"\0");
        hasher.update(&content);
    }
    Ok((hasher.digest(), files))
}

/// Per-file hashes: maps relative path (from project root) to xxh3 hash of content.
pub fn hash_sources_per_file(
    src_dir: &Path,
    project_root: &Path,
) -> Result<(HashMap<String, u64>, Vec<PathBuf>)> {
    let mut files: Vec<PathBuf> = WalkDir::new(src_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file()
                && e.path().extension().is_some_and(|ext| ext == "scala")
        })
        .map(|e| e.into_path())
        .collect();
    files.sort();

    let mut hashes = HashMap::new();
    for file in &files {
        let content = std::fs::read(file)?;
        let hash = xxhash_rust::xxh3::xxh3_64(&content);
        let rel = file
            .strip_prefix(project_root)
            .unwrap_or(file)
            .to_string_lossy()
            .to_string();
        hashes.insert(rel, hash);
    }
    Ok((hashes, files))
}

/// Diff two per-file hash maps. Returns (changed, added, deleted) relative paths.
pub fn diff_hashes(
    old: &HashMap<String, u64>,
    new: &HashMap<String, u64>,
) -> (Vec<String>, Vec<String>, Vec<String>) {
    let mut changed = Vec::new();
    let mut added = Vec::new();
    let mut deleted = Vec::new();

    for (path, new_hash) in new {
        match old.get(path) {
            Some(old_hash) if old_hash != new_hash => changed.push(path.clone()),
            None => added.push(path.clone()),
            _ => {}
        }
    }
    for path in old.keys() {
        if !new.contains_key(path) {
            deleted.push(path.clone());
        }
    }

    changed.sort();
    added.sort();
    deleted.sort();
    (changed, added, deleted)
}

pub fn read_cache(project_root: &Path, key: &str) -> Option<String> {
    let path = project_root.join(".sb/cache").join(key);
    std::fs::read_to_string(path).ok()
}

pub fn write_cache(project_root: &Path, key: &str, value: &str) -> Result<()> {
    let dir = project_root.join(".sb/cache");
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join(key), value)?;
    Ok(())
}
