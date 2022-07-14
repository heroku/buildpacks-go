use crate::vrs::{Requirement, RequirementParseError};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path;
use thiserror::Error;

/// Represents buildpack configuration found in a project's `go.mod`.
pub struct GoModCfg {
    pub packages: Option<Vec<String>>,
    pub version: Option<Requirement>,
}

#[derive(Error, Debug)]
pub enum ReadGoModCfgError {
    #[error("Failed to read go.mod configuration: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to parse go.mod configuration: {0}")]
    Version(#[from] RequirementParseError),
}

/// Build a `GoModCfg` from a `go.mod` file.
///
/// # Errors
///
/// Will return an error when the file cannot be read or the version strings
/// within are not parseable.
pub fn read_gomod_cfg<P: AsRef<path::Path>>(gomod_path: P) -> Result<GoModCfg, ReadGoModCfgError> {
    let mut version: Option<Requirement> = None;
    let mut packages: Option<Vec<String>> = None;
    let file = fs::File::open(gomod_path)?;
    for line_result in BufReader::new(file).lines() {
        let line = line_result?;
        let mut parts = line.split_whitespace();
        match (parts.next(), parts.next(), parts.next(), parts.next()) {
            (Some("//"), Some("+heroku"), Some("install"), Some(pkg)) => {
                let mut pkgs = vec![pkg.to_string()];
                for pkgn in parts {
                    pkgs.push(pkgn.to_string());
                }
                packages = Some(pkgs);
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
    Ok(GoModCfg { packages, version })
}
