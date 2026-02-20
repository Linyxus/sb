use anyhow::Result;
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
