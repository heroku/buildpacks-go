use heroku_go_utils::vrs::parse_go_version_requirement;

use std::fs;
use std::io::{BufRead, BufReader};
use std::path;

/// Represents buildpack configuration found in a project's `go.mod`.
#[derive(Debug)]
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
    go_config_reader(fs::File::open(gomod_path)?)
}

fn go_config_reader(buf: impl std::io::Read) -> Result<GoModConfig, ReadGoModConfigError> {
    let mut version: Option<semver::VersionReq> = None;
    let mut packages: Option<Vec<String>> = None;
    for line_result in BufReader::new(buf).lines() {
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

#[cfg(test)]
mod test {
    use super::*;
    use indoc::formatdoc;
    use semver::VersionReq;

    #[test]
    fn config_file_does_not_exist() {
        let result = read_gomod_config(path::Path::new(""));
        assert!(result.is_err(), "Expected {result:?} to err but it did not");
    }

    #[test]
    fn heroku_go_version_operator() {
        let go_mod = formatdoc! {"
            // +heroku goVersion =1.18.2
            go 1.17
        "};
        let GoModConfig { packages, version } = go_config_reader(go_mod.as_bytes()).unwrap();
        assert_eq!(None, packages);
        assert_eq!(Some(VersionReq::parse("=1.18.2").unwrap()), version);
    }

    #[test]
    fn go_version_without_heroku() {
        let go_mod = formatdoc! {"
            go 1.17
        "};
        let GoModConfig { packages, version } = go_config_reader(go_mod.as_bytes()).unwrap();
        assert_eq!(None, packages);
        assert_eq!(Some(VersionReq::parse("=1.17").unwrap()), version);
    }

    #[test]
    fn empty_file() {
        let go_mod = String::new();
        let GoModConfig { packages, version } = go_config_reader(go_mod.as_bytes()).unwrap();
        assert_eq!(packages, None);
        assert_eq!(version, None);

        let requirement = version.unwrap_or_default();
        assert_eq!(
            VersionReq {
                comparators: Vec::new()
            },
            requirement
        );
        assert_eq!("*".to_string(), requirement.to_string());
    }
}
