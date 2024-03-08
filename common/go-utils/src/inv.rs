use crate::vrs::{Requirement, Version, VersionParseError};
use serde::{Deserialize, Serialize};
use std::{env::consts, fs};
use toml;

const GO_RELEASES_URL: &str = "https://go.dev/dl/?mode=json&include=all";
const GO_HOST_URL: &str = "https://go.dev/dl";

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
    pub os: String,
    pub arch: String,
    pub url: String,
    pub sha_checksum: String,
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
            .filter(|artifact| artifact.os == consts::OS && artifact.arch == consts::ARCH)
            .find(|artifact| requirement.satisfies(&artifact.semantic_version))
    }
}

#[derive(Debug, Deserialize)]
struct GoRelease {
    files: Vec<GoFile>,
}

#[derive(Debug, Deserialize)]
struct GoFile {
    os: String,
    arch: String,
    filename: String,
    sha256: String,
    version: String,
}

impl TryFrom<&GoFile> for Artifact {
    type Error = VersionParseError;

    fn try_from(value: &GoFile) -> Result<Self, Self::Error> {
        let version = Version::parse_go(&value.version)?;
        let arch = match value.arch.as_str() {
            "amd64" => "x86_64".to_string(),
            _ => unimplemented!(),
        };

        Ok(Artifact {
            go_version: value.version.clone(),
            semantic_version: version,
            os: String::from("linux"),
            arch,
            sha_checksum: value.sha256.clone(),
            url: format!("{}/{}", GO_HOST_URL, value.filename),
        })
    }
}

/// List known go artifacts from releases on go.dev.
///
/// # Example
///
/// ```
/// let versions = heroku_go_utils::inv::list_upstream_artifacts().unwrap();
/// ```
///
/// # Errors
///
/// HTTP issues connecting to the upstream releases endpoint, as well
/// as json and Go version parsing issues, will return an error.
pub fn list_upstream_artifacts() -> Result<Vec<Artifact>, String> {
    ureq::get(GO_RELEASES_URL)
        .call()
        .map_err(|e| e.to_string())?
        .into_json::<Vec<GoRelease>>()
        .map_err(|e| e.to_string())?
        .iter()
        .flat_map(|release| &release.files)
        .filter(|file| !file.sha256.is_empty() && file.os == "linux" && file.arch == "amd64")
        .map(|file| Artifact::try_from(file).map_err(|e| e.to_string()))
        .collect::<Result<Vec<_>, _>>()
}
