use std::fmt;
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};

pub const MAVEN_CENTRAL: &str = "https://repo1.maven.org/maven2";

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct MavenCoord {
    pub group_id: String,
    pub artifact_id: String,
    pub version: String,
}

impl MavenCoord {
    pub fn new(group_id: impl Into<String>, artifact_id: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            group_id: group_id.into(),
            artifact_id: artifact_id.into(),
            version: version.into(),
        }
    }

    /// Parse "groupId:artifactId:version"
    pub fn parse(s: &str) -> Result<Self> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 3 {
            bail!("invalid Maven coordinate (expected group:artifact:version): {s}");
        }
        Ok(Self {
            group_id: parts[0].to_string(),
            artifact_id: parts[1].to_string(),
            version: parts[2].to_string(),
        })
    }

    /// e.g. "org/typelevel/cats-core_3/2.12.0"
    pub fn repo_path(&self) -> String {
        format!(
            "{}/{}/{}",
            self.group_id.replace('.', "/"),
            self.artifact_id,
            self.version
        )
    }

    fn filename(&self, ext: &str) -> String {
        format!("{}-{}.{}", self.artifact_id, self.version, ext)
    }

    pub fn pom_url(&self) -> String {
        format!("{}/{}/{}", MAVEN_CENTRAL, self.repo_path(), self.filename("pom"))
    }

    pub fn jar_url(&self) -> String {
        format!("{}/{}/{}", MAVEN_CENTRAL, self.repo_path(), self.filename("jar"))
    }

    pub fn local_pom_path(&self, cache_root: &Path) -> PathBuf {
        cache_root.join(self.repo_path()).join(self.filename("pom"))
    }

    pub fn local_jar_path(&self, cache_root: &Path) -> PathBuf {
        cache_root.join(self.repo_path()).join(self.filename("jar"))
    }

    /// Key for deduplication: (groupId, artifactId)
    pub fn key(&self) -> (String, String) {
        (self.group_id.clone(), self.artifact_id.clone())
    }
}

impl fmt::Display for MavenCoord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", self.group_id, self.artifact_id, self.version)
    }
}
