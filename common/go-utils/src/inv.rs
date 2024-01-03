use crate::vrs::{Requirement, Version, VersionParseError};
use serde::{Deserialize, Serialize};
use std::fs;
use toml;

const GITHUB_API_URL: &str = "https://api.github.com";
const GO_REPO_NAME: &str = "golang/go";
const GO_HOST_URL: &str = "https://dl.google.com/go";
const GO_MIRROR_URL: &str = "https://heroku-golang-prod.s3.us-east-1.amazonaws.com";
const ARCH: &str = "linux-amd64";

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

#[derive(thiserror::Error, Debug)]
pub enum ArtifactBuildError {
    #[error("Couldn't build Go artifact: {0}")]
    Checksum(#[from] FetchGoChecksumError),
    #[error("Couldn't build Go artifact: {0}")]
    Version(#[from] VersionParseError),
}

impl Artifact {
    /// Build an artifact from a go version.
    ///
    /// # Examples
    ///
    /// ```
    /// let art = heroku_go_utils::inv::Artifact::build("go1.16").unwrap();
    /// ```
    ///
    /// # Errors
    ///
    /// Will return an `Err` if the go version string is formatted incorrectly,
    /// or there is an http error fetching the checksum.
    pub fn build<S: Into<String>>(version: S) -> Result<Artifact, ArtifactBuildError> {
        let go_version: String = version.into();
        Ok(Artifact {
            semantic_version: Version::parse_go(&go_version)?,
            sha_checksum: fetch_go_checksum(&go_version)?,
            go_version,
            architecture: ARCH.to_string(),
        })
    }
}

#[derive(thiserror::Error, Debug)]
pub enum FetchGoChecksumError {
    #[error("Couldn't download Go checksum file: {0}")]
    Http(#[from] Box<ureq::Error>),
    #[error("Failed to read checksum value from Go checksum file: {0}")]
    Io(#[from] std::io::Error),
}
fn fetch_go_checksum(goversion: &str) -> Result<String, FetchGoChecksumError> {
    Ok(
        ureq::get(&format!("{GO_HOST_URL}/{goversion}.{ARCH}.tar.gz.sha256"))
            .call()
            .map_err(Box::new)?
            .into_string()?,
    )
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
/// let versions = heroku_go_utils::inv::list_github_go_versions().unwrap();
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
