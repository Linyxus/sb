use anyhow::{bail, Result};
use std::path::Path;

use crate::compile;
use crate::config::SbConfig;

pub fn run(config: &SbConfig, project_root: &Path, args: &[String]) -> Result<()> {
    let result = compile::compile(config, project_root)?;

    let main_class = config
        .project
        .main_class
        .as_deref()
        .unwrap_or_else(|| {
            // Default: project name capitalized
            // But we'll just require it for now
            ""
        });

    if main_class.is_empty() {
        bail!("no main-class specified in sb.toml");
    }

    let classes_dir = SbConfig::classes_dir(project_root);
    let runtime_cp = format!("{}:{}", classes_dir.display(), result.resolved.user_cp);

    // Use exec to replace process on Unix
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let err = std::process::Command::new("java")
            .arg("--sun-misc-unsafe-memory-access=allow")
            .arg("-cp")
            .arg(&runtime_cp)
            .arg(main_class)
            .args(args)
            .exec();
        // exec only returns on error
        bail!("failed to exec java: {err}");
    }

    #[cfg(not(unix))]
    {
        let status = std::process::Command::new("java")
            .arg("--sun-misc-unsafe-memory-access=allow")
            .arg("-cp")
            .arg(&runtime_cp)
            .arg(main_class)
            .args(args)
            .status()?;
        std::process::exit(status.code().unwrap_or(1));
    }
}
