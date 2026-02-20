use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
pub struct SbConfig {
    pub project: Project,
}

#[derive(Debug, Deserialize)]
pub struct Project {
    pub name: String,
    pub version: String,
    #[serde(rename = "scala-version")]
    pub scala_version: String,
    #[serde(rename = "main-class")]
    pub main_class: Option<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub scalac_options: Vec<String>,
}

impl SbConfig {
    pub fn load(project_root: &Path) -> Result<Self> {
        let path = project_root.join("sb.toml");
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let config: SbConfig =
            toml::from_str(&content).with_context(|| format!("failed to parse {}", path.display()))?;
        Ok(config)
    }

    pub fn source_dir(project_root: &Path) -> PathBuf {
        project_root.join("src/main/scala")
    }

    pub fn classes_dir(project_root: &Path) -> PathBuf {
        project_root.join(".sb/classes")
    }

    pub fn cache_dir(project_root: &Path) -> PathBuf {
        project_root.join(".sb/cache")
    }
}
