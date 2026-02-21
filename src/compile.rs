use anyhow::{bail, Result};
use std::path::Path;
use std::process::Command;

use crate::cache;
use crate::config::SbConfig;
use crate::resolve::{self, ResolvedClasspath};

pub struct CompileResult {
    pub resolved: ResolvedClasspath,
}

pub fn compile(config: &SbConfig, project_root: &Path) -> Result<CompileResult> {
    let src_dir = SbConfig::source_dir(project_root);
    let classes_dir = SbConfig::classes_dir(project_root);

    // Parallel: resolve deps + hash sources
    let (resolved, src_result) = std::thread::scope(|s| {
        let resolve_handle = s.spawn(|| resolve::resolve_classpath(config, project_root));
        let hash_handle = s.spawn(|| cache::hash_sources(&src_dir));
        (
            resolve_handle.join().expect("resolve thread panicked"),
            hash_handle.join().expect("hash thread panicked"),
        )
    });
    let resolved = resolved?;
    let (src_hash, sources) = src_result?;

    if sources.is_empty() {
        bail!("no .scala source files found in {}", src_dir.display());
    }

    let src_hash_str = src_hash.to_string();
    let dep_hash_str = resolve::dep_hash(config).to_string();

    // Check if recompilation is needed
    let cached_src = cache::read_cache(project_root, "src-hash");
    let cached_dep = cache::read_cache(project_root, "dep-hash");
    if cached_src.as_deref() == Some(&src_hash_str)
        && cached_dep.as_deref() == Some(&dep_hash_str)
        && classes_dir.exists()
        && std::fs::read_dir(&classes_dir)?.next().is_some()
    {
        eprintln!("Nothing to compile.");
        return Ok(CompileResult { resolved });
    }

    // Clean classes dir for fresh compile
    if classes_dir.exists() {
        std::fs::remove_dir_all(&classes_dir)?;
    }
    std::fs::create_dir_all(&classes_dir)?;

    eprintln!(
        "Compiling {} source file{}...",
        sources.len(),
        if sources.len() == 1 { "" } else { "s" }
    );

    let mut cmd = Command::new("java");
    cmd.arg("--sun-misc-unsafe-memory-access=allow");
    cmd.arg("-cp").arg(&resolved.compiler_cp);
    cmd.arg("dotty.tools.dotc.Main");
    cmd.arg("-classpath").arg(&resolved.user_cp);
    cmd.arg("-d").arg(&classes_dir);
    for opt in &config.project.scalac_options {
        cmd.arg(opt);
    }
    for src in &sources {
        cmd.arg(src);
    }

    let status = cmd.status()?;
    if !status.success() {
        bail!("compilation failed");
    }

    cache::write_cache(project_root, "src-hash", &src_hash_str)?;

    Ok(CompileResult { resolved })
}
