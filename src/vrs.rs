use anyhow::Context;
use regex::Regex;
use semver;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(try_from = "String", into = "String")]
pub struct Requirement(semver::VersionReq);
impl Requirement {
    pub fn parse(go_req: &str) -> anyhow::Result<Self> {
        let stripped_req = go_req.strip_prefix("go").unwrap_or(&go_req);
        let req = semver::VersionReq::parse(stripped_req).map(Self)?;
        Ok(req)
    }

    pub fn any() -> Self {
        Self(semver::VersionReq::any())
    }

    pub fn satisfies(&self, version: &Version) -> bool {
        self.0.matches(&version.0)
    }
}

impl TryFrom<String> for Requirement {
    type Error = anyhow::Error;
    fn try_from(val: String) -> Result<Self, Self::Error> {
        Requirement::parse(&val)
    }
}

impl From<Requirement> for String {
    fn from(req: Requirement) -> Self {
        format!("{req}")
    }
}

impl fmt::Display for Requirement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
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
        let stripped_version = go_version.strip_prefix("go").unwrap_or(&go_version);

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
            ("go1", "1.0.0"),
            ("1", "1.0.0"),
            ("go1.2", "1.2.0"),
            ("1.2", "1.2.0"),
            ("go1.2.3", "1.2.3"),
            ("1.2.3", "1.2.3"),
            ("go1.10.12", "1.10.12"),
            ("1.10.12", "1.10.12"),
            ("go1beta1", "1.0.0-beta1"),
            ("1beta1", "1.0.0-beta1"),
            ("go1.2rc3", "1.2.0-rc3"),
            ("go1.23.34alpha", "1.23.34-alpha"),
        ];

        for (input, expected_str) in go_versions {
            let actual = Version::parse_go(input).expect("Failed to parse go input version");
            let actual_str = actual.to_string();
            let expected =
                Version::parse(expected_str).expect("Failed to parse go expected version");
            assert_eq!(
                expected, actual,
                "Expected {input} to parse as {expected} but got {actual}."
            );
            assert_eq!(
                expected_str, actual_str,
                "Expected {input} to parse as {expected_str} but got {actual_str}"
            );
        }
    }

    #[test]
    fn test_requirement_parsing() {
        let examples = [
            ("go1", "^1"),
            ("1", "^1"),
            ("~1", "~1"),
            ("go1.16", "^1.16"),
            ("1.16", "^1.16"),
            ("~1.16", "~1.16"),
            ("go1.18.2", "^1.18.2"),
            ("1.18.2", "^1.18.2"),
            ("~1.18.2", "~1.18.2"),
        ];
        for (input, expected_str) in examples {
            let actual = Requirement::parse(input).expect("Failed to parse go input requirement");
            let actual_str = actual.to_string();
            let expected =
                Requirement::parse(expected_str).expect("Failed to parse go expected requirement");
            assert_eq!(
                expected, actual,
                "Expected {input} to parse as {expected} but got {actual}."
            );
            assert_eq!(
                expected_str, actual_str,
                "Expected {input} to parse as {expected_str} but got {actual_str}"
            );
        }
    }
}
