use crate::vrs::{Requirement, Version};
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::fs;
use toml;

pub const GITHUB_API_URL: &str = "https://api.github.com";
pub const GO_REPO_NAME: &str = "golang/go";
pub const GO_HOST_URL: &str = "https://dl.google.com/go";
pub const GO_MIRROR_URL: &str = "https://heroku-golang-prod.s3.amazonaws.com";
pub const REGION: &str = "us-east-1";
pub const ARCH: &str = "linux-amd64";

/// Represents a collection of known go release artifacts.
#[derive(Debug, Deserialize, Serialize)]
pub struct Inventory {
    pub artifacts: Vec<Artifact>,
}

/// Represents a known go release artifact in the inventory.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Artifact {
    pub go_version: String,
    pub semantic_version: Version,
    pub architecture: String,
    pub sha_checksum: String,
}

impl Artifact {
    #[must_use]
    pub fn mirror_tarball_url(&self) -> String {
        format!(
            "{}/{}.{}.tar.gz",
            GO_MIRROR_URL, self.go_version, self.architecture
        )
    }
}

#[derive(Debug)]
pub enum ArtifactError {
    Checksum(anyhow::Error),
    Version(anyhow::Error),
}

impl Artifact {
    /// Build an artifact from a go version.
    ///
    /// # Examples
    ///
    /// ```
    /// let art = heroku_go_buildpack::inv::Artifact::new("go1.16").unwrap();
    /// ```
    ///
    /// # Errors
    ///
    /// Will return an `Err` if the go version string is formatted incorrectly,
    /// or there is an http error fetching the checksum.
    pub fn new<S: Into<String>>(version: S) -> Result<Artifact, ArtifactError> {
        let go_version: String = version.into();
        let semantic_version = Version::parse_go(&go_version).map_err(ArtifactError::Version)?;
        let sha_checksum = fetch_go_checksum(&go_version).map_err(ArtifactError::Checksum)?;

        Ok(Artifact {
            sha_checksum,
            go_version,
            semantic_version,
            architecture: ARCH.to_string(),
        })
    }
}

fn fetch_go_checksum(goversion: &str) -> anyhow::Result<String> {
    let url = format!("{}/{}.{}.tar.gz.sha256", GO_HOST_URL, goversion, ARCH);
    ureq::get(&url)
        .call()
        .context(format!(
            "failed to download to remote checksum file from {url}"
        ))?
        .into_string()
        .context(format!("failed to read checksum value from {url}"))
}

impl Inventory {
    /// Read inventory.toml to an `Inventory`.
    ///
    /// # Errors
    ///
    /// Will return an Err if the file is missing, not readable, or if the
    /// file contents is not formatted properly.
    pub fn read(path: &str) -> anyhow::Result<Inventory> {
        let contents = fs::read_to_string(path)?;
        let inv = toml::from_str(&contents)?;
        Ok(inv)
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

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Tag {
    #[serde(alias = "ref")]
    reference: String,
}

/// List known go versions from tags on GitHub.
///
/// # Example
///
/// ```
/// let versions = heroku_go_buildpack::inv::list_github_go_versions().unwrap();
/// ```
///
/// # Errors
///
/// Http issues connecting to the GitHub tags endpoint will return an error.
pub fn list_github_go_versions() -> Result<Vec<String>, String> {
    let tag_names = ureq::get(&format!(
        "{GITHUB_API_URL}/repos/{GO_REPO_NAME}/git/refs/tags"
    ))
    .call()
    .map_err(|e| e.to_string())?
    .into_json::<Vec<Tag>>()
    .map_err(|e| e.to_string())?
    .iter()
    .filter_map(|t| t.reference.strip_prefix("refs/tags/"))
    .filter(|t| t.starts_with("go"))
    .map(std::string::ToString::to_string)
    .collect();
    Ok(tag_names)
}
