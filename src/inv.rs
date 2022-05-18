use anyhow::{anyhow, Context};
use regex::Regex;
use semver;
use serde::{Deserialize, Serialize};
use std::{fmt, fs};
use thiserror::Error;
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
    #[serde(alias = "version")]
    pub semantic_version: Version,
    #[serde(alias = "arch")]
    pub architecture: String,
    #[serde(alias = "sha")]
    pub sha_checksum: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(try_from = "String", into = "String")]
pub struct Version(semver::Version);
impl Version {
    /// Parses a semver string as a `Version`
    ///
    /// # Errors
    ///
    /// Invalid semver strings will return a `VersionError`
    pub fn parse(version: &str) -> Result<Self, VersionError> {
        let trimmed = version.trim();
        match semver::Version::parse(trimmed) {
            Ok(v) => Ok(Version(v)),
            Err(e) => Err(VersionError(format!("{}", e))),
        }
    }
}
impl TryFrom<String> for Version {
    type Error = VersionError;
    fn try_from(val: String) -> Result<Self, Self::Error> {
        Version::parse(&val)
    }
}
impl From<Version> for String {
    fn from(ver: Version) -> Self {
        format!("{ver}")
    }
}
impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Error, Debug)]
#[error("Error with Semantic Version: {0}")]
pub struct VersionError(String);

pub enum ArtifactError {
    Checksum(anyhow::Error),
    SemVer(anyhow::Error),
}

impl Artifact {
    pub fn new<S: Into<String>>(goversion: S) -> Result<Artifact, ArtifactError> {
        let go_version: String = goversion.into();
        let semantic_version = parse_go_semver(&go_version).map_err(ArtifactError::SemVer)?;
        let sha_checksum = fetch_go_checksum(&go_version).map_err(ArtifactError::Checksum)?;

        Ok(Artifact {
            sha_checksum,
            go_version,
            semantic_version,
            architecture: ARCH.to_string(),
        })
    }
}

pub fn parse_go_semver(goversion: &str) -> anyhow::Result<Version> {
    let stripped_version = goversion
        .strip_prefix("go")
        .ok_or(anyhow!("missing go prefix for {goversion}"))?;

    let re = Regex::new(r"^(\d+)\.?(\d+)?\.?(\d+)?([a-z][a-z0-9]*)?$")?;
    let caps = re
        .captures(stripped_version)
        .context(format!("couldn't find version identifiers in {goversion}"))?;

    let mut composed_version = vec![
        caps.get(1).map(|major| major.as_str()).unwrap_or("0"),
        caps.get(2).map(|minor| minor.as_str()).unwrap_or("0"),
        caps.get(3).map(|patch| patch.as_str()).unwrap_or("0"),
    ]
    .join(".");

    if let Some(pre) = caps.get(4) {
        composed_version.push('-');
        composed_version.push_str(pre.as_str());
    };

    Version::parse(&composed_version).context(format!("couldn't parse semver for {goversion}"))
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
    pub fn read(path: &str) -> Result<Inventory, String> {
        let contents = fs::read_to_string(path).map_err(|e| e.to_string())?;
        toml::from_str(&contents).map_err(|e| e.to_string())
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Tag {
    #[serde(alias = "ref")]
    reference: String,
}

/// List known go versions from tags on GitHub.
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
    .map(|v| v.to_string())
    .collect();
    Ok(tag_names)
}
