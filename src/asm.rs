use anyhow::Result;
use std::collections::HashSet;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use zip::write::FileOptions;
use zip::ZipWriter;

use crate::compile;
use crate::config::SbConfig;

pub fn assemble(config: &SbConfig, project_root: &Path) -> Result<PathBuf> {
    let main_class = config
        .project
        .main_class
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("main-class is required in sb.toml for assembly"))?;

    let result = compile::compile(config, project_root)?;

    let output_path = project_root
        .join(".sb")
        .join(format!("{}-{}-assembly.jar", config.project.name, config.project.version));

    let file = std::fs::File::create(&output_path)?;
    let mut zip = ZipWriter::new(file);
    let mut seen = HashSet::new();

    let options = FileOptions::<()>::default()
        .compression_method(zip::CompressionMethod::Deflated);

    // Write manifest
    let manifest = format!(
        "Manifest-Version: 1.0\r\nMain-Class: {}\r\n\r\n",
        main_class
    );
    let manifest_path = "META-INF/MANIFEST.MF";
    zip.start_file(manifest_path, options)?;
    zip.write_all(manifest.as_bytes())?;
    seen.insert(manifest_path.to_string());
    seen.insert("META-INF/".to_string());

    // Add compiled classes
    let classes_dir = SbConfig::classes_dir(project_root);
    for entry in WalkDir::new(&classes_dir) {
        let entry = entry?;
        if entry.file_type().is_dir() {
            continue;
        }
        let rel = entry
            .path()
            .strip_prefix(&classes_dir)?
            .to_string_lossy()
            .replace('\\', "/");
        if seen.contains(&rel) {
            continue;
        }
        zip.start_file(&rel, options)?;
        let data = std::fs::read(entry.path())?;
        zip.write_all(&data)?;
        seen.insert(rel);
    }

    // Merge dependency JARs
    for jar_path in result.resolved.user_cp.split(':') {
        if jar_path.is_empty() || !jar_path.ends_with(".jar") {
            continue;
        }
        let jar_file = match std::fs::File::open(jar_path) {
            Ok(f) => f,
            Err(_) => continue,
        };
        let mut archive = match zip::ZipArchive::new(jar_file) {
            Ok(a) => a,
            Err(_) => continue,
        };
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;
            let name = entry.name().to_string();

            // Skip directories, manifests, and signature files
            if name.ends_with('/') {
                continue;
            }
            if name == "META-INF/MANIFEST.MF" {
                continue;
            }
            if name.starts_with("META-INF/")
                && (name.ends_with(".SF") || name.ends_with(".RSA") || name.ends_with(".DSA"))
            {
                continue;
            }
            if seen.contains(&name) {
                continue;
            }

            zip.start_file(&name, options)?;
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf)?;
            zip.write_all(&buf)?;
            seen.insert(name);
        }
    }

    zip.finish()?;
    Ok(output_path)
}
