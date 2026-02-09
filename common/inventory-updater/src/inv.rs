use heroku_go_utils::vrs::{GoVersion, GoVersionParseError};
use libherokubuildpack::inventory::{
    artifact::{Arch, Artifact, Os, UnsupportedArchError, UnsupportedOsError},
    checksum::{self, Checksum},
};
use serde::Deserialize;
use sha2::Sha256;

const GO_RELEASES_URL: &str = "https://go.dev/dl/?mode=json&include=all";
const GO_HOST_URL: &str = "https://go.dev/dl";

#[derive(Debug, Deserialize)]
struct GoRelease {
    version: GoVersion,
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
pub(crate) enum GoFileConversionError {
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
pub(crate) enum ListUpstreamArtifactsError {
    #[error("Invalid response fetching {0}")]
    InvalidResponse(Box<ureq::Error>),
    #[error(transparent)]
    ParseJsonResponse(Box<ureq::Error>),
    #[error(transparent)]
    Conversion(#[from] GoFileConversionError),
    #[error("Go version {version} is missing the linux/{arch} artifact")]
    MissingArtifact { version: GoVersion, arch: String }, // New variant
}

/// List known go artifacts from releases on go.dev.
///
/// # Example
///
/// ```no_run
/// let versions = inventory_updater::upstream::list_upstream_artifacts().unwrap();
/// ```
///
/// # Errors
///
/// HTTP issues connecting to the upstream releases endpoint, as well
/// as json and Go version parsing issues, will return an error.
pub(crate) fn list_upstream_artifacts()
-> Result<Vec<Artifact<GoVersion, Sha256, Option<()>>>, ListUpstreamArtifactsError> {
    let releases: Vec<GoRelease> = ureq::get(GO_RELEASES_URL)
        .call()
        .map_err(|e| ListUpstreamArtifactsError::InvalidResponse(Box::new(e)))?
        .body_mut()
        .read_json()
        .map_err(|e| ListUpstreamArtifactsError::ParseJsonResponse(Box::new(e)))?;

    let min_version = GoVersion::try_from("go1.5.3".to_string())
        .expect("Minimum supported version should always be parseable");
    let min_arm_version = GoVersion::try_from("go1.8.5".to_string())
        .expect("Minimum supported ARM version should always be parseable");

    // Note: `go1.5.3`` (the earliest supported version) and up include checksums for all files,
    // with the notable exception of `go1.6beta1`.
    // TODO: Drop this if/when we remove support for <go1.6.0
    let excluded_versions = [GoVersion::try_from("go1.6beta1".to_string())
        .expect("Excluded version should always be parseable")];

    releases
        .into_iter()
        .filter(|release| {
            release.version >= min_version && !excluded_versions.contains(&release.version)
        })
        .flat_map(|release| {
            let required_archs = if release.version >= min_arm_version {
                vec!["amd64", "arm64"]
            } else {
                vec!["amd64"]
            };

            required_archs.into_iter().map(move |arch| {
                release
                    .files
                    .iter()
                    .find(|file| file.os == "linux" && file.arch == arch)
                    .ok_or_else(|| ListUpstreamArtifactsError::MissingArtifact {
                        version: release.version.clone(),
                        arch: arch.to_string(),
                    })
                    .and_then(|file| Ok(Artifact::try_from(file)?))
            })
        })
        .collect()
}
