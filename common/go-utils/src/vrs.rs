use regex::Regex;
use semver;
use serde::{Deserialize, Serialize};
use std::fmt;

/// `Requirement` is a wrapper around `semver::Requirement` that adds
/// - Ability to parse go-flavored requirements
///
/// The derived `Default` implementation creates a wildcard version `Requirement`.
#[derive(Debug, Default, PartialEq)]
pub struct Requirement(semver::VersionReq);

#[derive(thiserror::Error, Debug)]
#[error("Couldn't parse Go version requirement: {0}")]
pub struct RequirementParseError(#[from] semver::Error);

impl Requirement {
    /// Parses a semver requirement `&str` as a `Requirement`.
    ///
    /// # Examples
    ///
    /// ```
    /// let req = heroku_go_utils::vrs::Requirement::parse("~1.0").unwrap();
    /// ```
    ///
    /// # Errors
    /// Invalid semver requirement `&str` like ">< 1.0", ".1.0", "!=4", etc.
    /// will return an error.
    pub fn parse(input: &str) -> Result<Self, RequirementParseError> {
        semver::VersionReq::parse(input)
            .map(Self)
            .map_err(RequirementParseError)
    }

    /// Parses a go version requirement `&str` as a `Requirement`
    ///
    /// # Examples
    ///
    /// ```
    /// let req = heroku_go_utils::vrs::Requirement::parse_go("go1.0").unwrap();
    /// ```
    ///
    /// # Errors
    /// Invalid semver requirement `&str` like ">< 1.0", ".1.0", "!=4", etc.
    /// will return an error.
    pub fn parse_go(go_req: &str) -> Result<Self, RequirementParseError> {
        go_req
            .strip_prefix("go")
            .map_or(Self::parse(go_req), |req| {
                Self::parse(format!("={req}").as_str())
            })
    }

    /// Determines if a `&Version` satisfies a `Requirement`
    #[must_use]
    pub fn satisfies(&self, version: &Version) -> bool {
        self.0.matches(&version.0)
    }
}

impl fmt::Display for Requirement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// `Version` is a wrapper around `semver::Version` that adds
/// - `Deserialize` and `Serialize` traits
/// - Ability to parse go-flavored versions
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(try_from = "String", into = "String")]
pub struct Version(semver::Version);

#[derive(thiserror::Error, Debug)]
pub enum VersionParseError {
    #[error("Couldn't parse go version: {0}")]
    SemVer(#[from] semver::Error),
    #[error("Internal buildpack issue parsing go version regex: {0}")]
    Regex(#[from] regex::Error),
    #[error("Couldn't parse version. Unable to capture values from regex.")]
    Captures,
}

impl Version {
    /// Parses a semver `&str` as a `Version`
    ///
    /// # Examples
    ///
    /// ```
    /// let req = heroku_go_utils::vrs::Version::parse("1.14.2").unwrap();
    /// ```
    ///
    /// # Errors
    ///
    /// Invalid semver `&str`s like ".1", "1.*", "abc", etc. will return an error.
    pub fn parse(version: &str) -> Result<Version, VersionParseError> {
        semver::Version::parse(version)
            .map(Version)
            .map_err(VersionParseError::SemVer)
    }

    /// Parses a go version `&str` as a `Version`
    ///
    /// # Examples
    ///
    /// ```
    /// let req = heroku_go_utils::vrs::Version::parse_go("go1.12").unwrap();
    /// ```
    ///
    /// # Errors
    ///
    /// Invalid go version `&str`s like ".1", "1.*", "abc", etc. will return an error.
    pub fn parse_go(go_version: &str) -> Result<Version, VersionParseError> {
        let stripped_version = go_version.strip_prefix("go").unwrap_or(go_version);

        let re = Regex::new(r"^(\d+)\.?(\d+)?\.?(\d+)?([a-z][a-z0-9]*)?$")?;
        let caps = re
            .captures(stripped_version)
            .ok_or(VersionParseError::Captures)?;

        let mut composed_version = [
            caps.get(1).map_or("0", |major| major.as_str()),
            caps.get(2).map_or("0", |minor| minor.as_str()),
            caps.get(3).map_or("0", |patch| patch.as_str()),
        ]
        .join(".");

        if let Some(pre) = caps.get(4) {
            composed_version.push('-');
            composed_version.push_str(pre.as_str());
        };

        Version::parse(&composed_version)
    }
}

impl TryFrom<String> for Version {
    type Error = VersionParseError;
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
            ("go1", "=1"),
            ("1", "^1"),
            ("=1", "=1"),
            ("go1.16", "=1.16"),
            ("1.16", "^1.16"),
            ("~1.16", "~1.16"),
            ("go1.18.2", "=1.18.2"),
            ("1.18.2", "^1.18.2"),
            ("^1.18.2", "^1.18.2"),
        ];
        for (input, expected_str) in examples {
            let actual = Requirement::parse_go(input)
                .unwrap_or_else(|_| panic!("Failed to parse go input requirement: {input}"));
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
