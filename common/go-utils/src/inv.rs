use crate::vrs::{GoVersion, GoVersionParseError};
use heroku_inventory_utils::checksum::{Algorithm, Checksum, Error as ChecksumError};
use heroku_inventory_utils::inv::{
    Arch, Artifact, Inventory, Os, UnsupportedArchError, UnsupportedOsError,
};
use heroku_inventory_utils::upstream::UpstreamInventory;
use heroku_inventory_utils::vrs::Version;
use serde::Deserialize;

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
    Checksum(#[from] ChecksumError),
}

impl TryFrom<&GoFile> for Artifact<GoVersion> {
    type Error = GoFileConversionError;

    fn try_from(value: &GoFile) -> Result<Self, Self::Error> {
        Ok(Self {
            version: GoVersion::parse(&value.version)?,
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

impl UpstreamInventory<GoVersion> for Inventory<GoVersion> {
    type Error = ListUpstreamArtifactsError;

    fn list_upstream_artifacts(
    ) -> Result<std::collections::HashSet<Artifact<GoVersion>>, Self::Error> {
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
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_artifact() -> Artifact<GoVersion> {
        Artifact::<GoVersion> {
            version: GoVersion::parse("1.7.2").unwrap(),
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

        assert_eq!("Go 1.7.2 (linux-x86_64)", artifact.to_string());
    }
}
