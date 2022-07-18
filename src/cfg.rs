use heroku_go_buildpack::vrs::{Requirement, RequirementParseError};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path;

/// Represents buildpack configuration found in a project's `go.mod`.
pub struct GoModConfig {
    pub packages: Option<Vec<String>>,
    pub version: Option<Requirement>,
}

#[derive(thiserror::Error, Debug)]
pub enum ReadGoModConfigError {
    #[error("Failed to read go.mod configuration: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to parse go.mod configuration: {0}")]
    Version(#[from] RequirementParseError),
}

/// Build a `GoModConfig` from a `go.mod` file.
///
/// # Errors
///
/// Will return an error when the file cannot be read or the version strings
/// within are not parseable.
pub fn read_gomod_config<P: AsRef<path::Path>>(
    gomod_path: P,
) -> Result<GoModConfig, ReadGoModConfigError> {
    let mut version: Option<Requirement> = None;
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
                version = Requirement::parse_go(vrs).map(Some)?;
            }
            (Some("go"), Some(vrs), None, None) => {
                if version == None {
                    version = Requirement::parse_go(&format!("={vrs}")).map(Some)?;
                }
            }
            _ => (),
        }
    }
    Ok(GoModConfig { packages, version })
}
