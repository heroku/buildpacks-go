use crate::vrs::{GoVersion, GoVersionParseError};
use core::fmt::{self, Display};
use heroku_inventory_utils::checksum::{Algorithm, Checksum, Error as ChecksumError};
use heroku_inventory_utils::vrs::{Version, VersionRequirement};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::hash::Hash;
use std::{env::consts, fs, str::FromStr};
use toml;

const GO_RELEASES_URL: &str = "https://go.dev/dl/?mode=json&include=all";
const GO_HOST_URL: &str = "https://go.dev/dl";

/// Represents a collection of known go release artifacts.
#[derive(Debug, Deserialize, Serialize)]
pub struct Inventory<V>
where
    V: Version,
{
    pub artifacts: Vec<Artifact<V>>,
}
/// Represents a known artifact in the inventory.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct Artifact<V>
where
    V: Version,
{
    version: String,
    pub semantic_version: V,
    os: Os,
    arch: Arch,
    pub url: String,
    pub checksum: Checksum,
}

impl<V: Version> Hash for Artifact<V> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.checksum.value.hash(state);
    }
}

impl<V: Version> Display for Artifact<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({}-{})", self.version, self.os, self.arch)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum Os {
    Linux,
}

impl Display for Os {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Os::Linux => write!(f, "linux"),
        }
    }
}

#[derive(thiserror::Error, Debug)]
#[error("OS is not supported: {0}")]
pub struct UnsupportedOsError(String);

impl FromStr for Os {
    type Err = UnsupportedOsError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "linux" => Ok(Os::Linux),
            _ => Err(UnsupportedOsError(s.to_string())),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum Arch {
    X86_64,
    Aarch64,
}

impl Display for Arch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Arch::X86_64 => write!(f, "x86_64"),
            Arch::Aarch64 => write!(f, "aarch64"),
        }
    }
}

#[derive(thiserror::Error, Debug)]
#[error("Arch is not supported: {0}")]
pub struct UnsupportedArchError(String);

impl FromStr for Arch {
    type Err = UnsupportedArchError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "amd64" | "x86_64" => Ok(Arch::X86_64),
            "arm64" | "aarch64" => Ok(Arch::Aarch64),
            _ => Err(UnsupportedArchError(s.to_string())),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ReadInventoryError {
    #[error("Couldn't read Go artifact inventory.toml: {0}")]
    Io(#[from] std::io::Error),
    #[error("Couldn't parse Go artifact inventory.toml: {0}")]
    Parse(#[from] toml::de::Error),
}

impl<V> Inventory<V>
where
    V: Version + DeserializeOwned,
{
    /// Read inventory.toml to an `Inventory<V>`.
    ///
    /// # Errors
    ///
    /// Will return an Err if the file is missing, not readable, or if the
    /// file contents is not formatted properly.
    pub fn read(path: &str) -> Result<Self, ReadInventoryError> {
        toml::from_str(&fs::read_to_string(path)?).map_err(ReadInventoryError::Parse)
    }

    /// Find the first artifact from the inventory that satisfies a
    /// `Requirement`.
    #[must_use]
    pub fn resolve<R>(&self, requirement: &R) -> Option<&Artifact<V>>
    where
        R: VersionRequirement<V>,
    {
        match (consts::OS.parse::<Os>(), consts::ARCH.parse::<Arch>()) {
            (Ok(os), Ok(arch)) => self
                .artifacts
                .iter()
                .filter(|artifact| artifact.os == os && artifact.arch == arch)
                .find(|artifact| requirement.satisfies(&artifact.semantic_version)),
            (_, _) => None,
        }
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

#[derive(thiserror::Error, Debug)]
pub enum GoFileConversionError {
    #[error(transparent)]
    Version(#[from] GoVersionParseError),
    #[error(transparent)]
    Arch(#[from] UnsupportedArchError),
    #[error(transparent)]
    Os(#[from] UnsupportedOsError),
    #[error(transparent)]
    Checksum(#[from] ChecksumError),
}

impl TryFrom<&GoFile> for Artifact<GoVersion> {
    type Error = GoFileConversionError;

    fn try_from(value: &GoFile) -> Result<Self, Self::Error> {
        Ok(Self {
            version: value.version.clone(),
            semantic_version: GoVersion::parse_go(&value.version)?,
            os: value.os.parse::<Os>()?,
            arch: value.arch.parse::<Arch>()?,
            checksum: Checksum::new(Algorithm::Sha256, value.sha256.to_string())?,
            url: format!("{}/{}", GO_HOST_URL, value.filename),
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ListUpstreamArtifactsError {
    #[error("Invalid response fetching {0}")]
    InvalidResponse(Box<ureq::Error>),
    #[error(transparent)]
    ParseJsonResponse(std::io::Error),
    #[error(transparent)]
    Conversion(#[from] GoFileConversionError),
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
pub fn list_upstream_artifacts() -> Result<Vec<Artifact<GoVersion>>, ListUpstreamArtifactsError> {
    ureq::get(GO_RELEASES_URL)
        .call()
        .map_err(|e| ListUpstreamArtifactsError::InvalidResponse(Box::new(e)))?
        .into_json::<Vec<GoRelease>>()
        .map_err(ListUpstreamArtifactsError::ParseJsonResponse)?
        .iter()
        .flat_map(|release| &release.files)
        .filter(|file| {
            !file.sha256.is_empty()
                && file.os == "linux"
                && (file.arch == "amd64" || file.arch == "arm64")
        })
        .map(|file| Artifact::try_from(file).map_err(ListUpstreamArtifactsError::Conversion))
        .collect::<Result<Vec<_>, _>>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::hash::{BuildHasher, RandomState};

    #[test]
    fn test_arch_display_format() {
        let archs = [(Arch::X86_64, "x86_64"), (Arch::Aarch64, "aarch64")];

        for (input, expected) in archs {
            assert_eq!(expected, input.to_string());
        }
    }

    #[test]
    fn test_arch_parsing() {
        let archs = [
            ("amd64", Arch::X86_64),
            ("arm64", Arch::Aarch64),
            ("x86_64", Arch::X86_64),
            ("aarch64", Arch::Aarch64),
        ];
        for (input, expected) in archs {
            assert_eq!(expected, input.parse::<Arch>().unwrap());
        }

        assert!(matches!(
            "foo".parse::<Arch>().unwrap_err(),
            UnsupportedArchError(..)
        ));
    }

    #[test]
    fn test_os_display_format() {
        assert_eq!("linux", Os::Linux.to_string());
    }

    #[test]
    fn test_os_parsing() {
        assert_eq!(Os::Linux, "linux".parse::<Os>().unwrap());

        assert!(matches!(
            "foo".parse::<Os>().unwrap_err(),
            UnsupportedOsError(..)
        ));
    }

    fn create_artifact() -> Artifact<GoVersion> {
        Artifact::<GoVersion> {
            version: String::from("go1.7.2"),
            semantic_version: GoVersion::parse("1.7.2").unwrap(),
            os: Os::Linux,
            arch: Arch::X86_64,
            url: String::from("foo"),
            checksum: Checksum::new(
                Algorithm::Sha256,
                "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string(),
            )
            .unwrap(),
        }
    }

    #[test]
    fn test_artifact_display_format() {
        let artifact = create_artifact();

        assert_eq!("go1.7.2 (linux-x86_64)", artifact.to_string());
    }

    #[test]
    fn test_artifact_hash_implementation() {
        let artifact = create_artifact();

        let state = RandomState::new();
        assert_eq!(
            state.hash_one(&artifact.checksum.value),
            state.hash_one(&artifact)
        );
    }

    #[test]
    fn test_artifact_serialization() {
        let artifact = create_artifact();
        let serialized = toml::to_string(&artifact).unwrap();
        assert!(serialized
            .contains("sha256:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"));
        assert_eq!(
            artifact,
            toml::from_str::<Artifact<GoVersion>>(&serialized).unwrap()
        );
    }
}
