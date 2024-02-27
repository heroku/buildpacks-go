use crate::vrs::{Requirement, Version};
use serde::{Deserialize, Serialize};
use std::fs;
use toml;

const GO_RELEASES_URL: &str = "https://go.dev/dl/?mode=json&include=all";
const GO_HOST_URL: &str = "https://dl.google.com/go";
const ARCH: &str = "linux-amd64";

/// Represents a collection of known go release artifacts.
#[derive(Debug, Deserialize, Serialize)]
pub struct Inventory {
    pub artifacts: Vec<Artifact>,
}

/// Represents a known go release artifact in the inventory.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct Artifact {
    pub go_version: String,
    pub semantic_version: Version,
    pub architecture: String,
    pub sha_checksum: String,
}

impl Artifact {
    #[must_use]
    pub fn tarball_url(&self) -> String {
        format!(
            "{}/{}.{}.tar.gz",
            GO_HOST_URL, self.go_version, self.architecture
        )
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ReadInventoryError {
    #[error("Couldn't read Go artifact inventory.toml: {0}")]
    Io(#[from] std::io::Error),
    #[error("Couldn't parse Go artifact inventory.toml: {0}")]
    Parse(#[from] toml::de::Error),
}

impl Inventory {
    /// Read inventory.toml to an `Inventory`.
    ///
    /// # Errors
    ///
    /// Will return an Err if the file is missing, not readable, or if the
    /// file contents is not formatted properly.
    pub fn read(path: &str) -> Result<Self, ReadInventoryError> {
        Ok(toml::from_str(&fs::read_to_string(path)?)?)
    }

    /// Find the first artifact from the inventory that satisfies a
    /// `Requirement`.
    #[must_use]
    pub fn resolve(&self, requirement: &Requirement) -> Option<&Artifact> {
        self.artifacts
            .iter()
            .find(|artifact| requirement.satisfies(&artifact.semantic_version))
    }
}

#[derive(Debug, Deserialize)]
struct GoRelease {
    version: String,
    files: Vec<GoFile>,
}

impl GoRelease {
    fn get_go_release_file(&self) -> Option<&GoFile> {
        self.files
            .iter()
            .filter(|f| !f.sha256.is_empty() && ARCH == f.get_target_arch())
            .nth(0)
    }
}
#[derive(Debug, Deserialize)]
struct GoFile {
    os: String,
    arch: String,
    sha256: String,
}

impl GoFile {
    fn get_target_arch(&self) -> String {
        format!("{}-{}", self.os, self.arch)
    }
}

/// List known go artifacts from releases on gov.dev.
///
/// # Example
///
/// ```
/// let versions = heroku_go_utils::inv::list_upstream_artifacts().unwrap();
/// ```
///
/// # Errors
///
/// Http issues connecting to the Go releases endpoint will return an error.
pub fn list_upstream_artifacts() -> Result<Vec<Artifact>, String> {
    let artifacts = ureq::get(GO_RELEASES_URL)
        .call()
        .map_err(|e| e.to_string())?
        .into_json::<Vec<GoRelease>>()
        .map_err(|e| e.to_string())?
        .iter()
        .filter_map(|t| {
            t.get_go_release_file().map(|gofile| Artifact {
                go_version: t.version.clone(),
                semantic_version: Version::parse_go(&t.version).unwrap_or_else(|e| {
                    eprintln!("Error parsing artifact version '{}': {e}", t.version);
                    std::process::exit(1);
                }),
                architecture: gofile.get_target_arch(),
                sha_checksum: gofile.sha256.clone(),
            })
        })
        .collect();
    Ok(artifacts)
}
