use crate::vrs;
use libherokubuildpack::inventory::{
    artifact::{Arch, Artifact, Os, UnsupportedArchError, UnsupportedOsError},
    checksum::{self, Checksum},
};
use serde::Deserialize;
use sha2::Sha256;
use vrs::{GoVersion, GoVersionParseError};

const GO_RELEASES_URL: &str = "https://go.dev/dl/?mode=json&include=all";
const GO_HOST_URL: &str = "https://go.dev/dl";

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
    Checksum(#[from] checksum::ChecksumParseError),
}

impl TryFrom<&GoFile> for Artifact<GoVersion, Sha256, Option<()>> {
    type Error = GoFileConversionError;

    fn try_from(value: &GoFile) -> Result<Self, Self::Error> {
        Ok(Artifact {
            version: value.version.clone().try_into()?,
            os: value.os.parse::<Os>()?,
            arch: value.arch.parse::<Arch>()?,
            checksum: format!("sha256:{}", value.sha256.clone()).parse::<Checksum<Sha256>>()?,
            url: format!("{}/{}", GO_HOST_URL, value.filename),
            metadata: None,
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
pub fn list_upstream_artifacts()
-> Result<Vec<Artifact<GoVersion, Sha256, Option<()>>>, ListUpstreamArtifactsError> {
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

    fn create_artifact() -> Artifact<GoVersion, Sha256, Option<()>> {
        Artifact {
            version: GoVersion::try_from("1.7.2".to_string()).unwrap(),
            os: Os::Linux,
            arch: Arch::Amd64,
            url: String::from("foo"),
            checksum: "sha256:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
                .parse::<Checksum<Sha256>>()
                .unwrap(),
            metadata: None,
        }
    }

    #[test]
    fn test_artifact_serialization() {
        let artifact = create_artifact();
        let serialized = toml::to_string(&artifact).unwrap();
        assert!(
            serialized.contains(
                "sha256:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
            )
        );
        assert_eq!(
            artifact,
            toml::from_str::<Artifact<GoVersion, Sha256, Option<()>>>(&serialized).unwrap()
        );
    }
}
