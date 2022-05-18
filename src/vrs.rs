use anyhow::{anyhow, Context};
use regex::Regex;
use semver;
use serde::{Deserialize, Serialize};
use std::fmt;

/// `Version` is a wrapper around semver::Version that adds
/// - `Deserialize` and `Serialize` traits
/// - Ability to parse go-flavored versions
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(try_from = "String", into = "String")]
pub struct Version(semver::Version);
impl Version {
    /// Parses a semver string as a `Version`
    ///
    /// # Errors
    ///
    /// Invalid semver strings will return an error
    pub fn parse(version: &str) -> anyhow::Result<Version> {
        let vrs = semver::Version::parse(version.trim()).map(Version)?;
        Ok(vrs)
    }

    /// Parses a go version (like go1.1) as a `Version`
    ///
    /// # Errors
    ///
    /// Invalid go version strings will return an error
    pub fn parse_go(go_version: &str) -> anyhow::Result<Version> {
        let stripped_version = go_version
            .strip_prefix("go")
            .ok_or(anyhow!("missing go prefix for {go_version}"))?;

        let re = Regex::new(r"^(\d+)\.?(\d+)?\.?(\d+)?([a-z][a-z0-9]*)?$")?;
        let caps = re
            .captures(stripped_version)
            .context(format!("couldn't find version identifiers in {go_version}"))?;

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

        Version::parse(&composed_version).context(format!("couldn't parse semver for {go_version}"))
    }
}

impl TryFrom<String> for Version {
    type Error = anyhow::Error;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parsing() {
        let go_versions = [
            ["go1", "1.0.0"],
            ["go1.2", "1.2.0"],
            ["go1.2.3", "1.2.3"],
            ["go1.10.12", "1.10.12"],
            ["go1beta1", "1.0.0-beta1"],
            ["go1.2rc3", "1.2.0-rc3"],
            ["go1.23.34alpha", "1.23.34-alpha"],
        ];

        for [gover, semver] in go_versions {
            let parsed_gover = Version::parse_go(gover).expect("Failed to parse go version");
            assert_eq!(semver, parsed_gover.to_string());
            let parsed_semver = Version::parse(semver).expect("Failed to parse semantic version");
            assert_eq!(parsed_gover, parsed_semver);
        }
    }
}
