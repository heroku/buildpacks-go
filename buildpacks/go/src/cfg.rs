use heroku_go_utils::vrs::parse_go_version_requirement;

use std::fs;
use std::io::{BufRead, BufReader};
use std::path;

/// Represents buildpack configuration found in a project's `go.mod`.
pub(crate) struct GoModConfig {
    pub(crate) packages: Option<Vec<String>>,
    pub(crate) version: Option<semver::VersionReq>,
}

#[derive(thiserror::Error, Debug)]
pub(crate) enum ReadGoModConfigError {
    #[error("Failed to read go.mod configuration: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to parse go.mod configuration: {0}")]
    Version(#[from] semver::Error),
}

/// Build a `GoModConfig` from a `go.mod` file.
///
/// # Errors
///
/// Will return an error when the file cannot be read or the version strings
/// within are not parseable.
pub(crate) fn read_gomod_config<P: AsRef<path::Path>>(
    gomod_path: P,
) -> Result<GoModConfig, ReadGoModConfigError> {
    let mut version: Option<semver::VersionReq> = None;
    let mut packages: Option<Vec<String>> = None;
    let file = fs::File::open(gomod_path)?;
    for line_result in BufReader::new(file).lines() {
        let line = line_result?;
        let mut parts = line.split_whitespace().peekable();
        match (parts.next(), parts.next(), parts.next(), parts.peek()) {
            (Some("//"), Some("+heroku"), Some("install"), Some(_)) => {
                packages = Some(parts.map(ToString::to_string).collect());
            }
            (Some("//"), Some("+heroku"), Some("goVersion"), Some(vrs)) => {
                version = parse_go_version_requirement(vrs).map(Some)?;
            }
            (Some("go"), Some(vrs), None, None) => {
                if version.is_none() {
                    version = parse_go_version_requirement(&format!("={vrs}")).map(Some)?;
                }
            }
            _ => (),
        }
    }
    Ok(GoModConfig { packages, version })
}
