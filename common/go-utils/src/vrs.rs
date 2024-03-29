use heroku_inventory_utils::vrs::{RequirementParseError, Version, VersionRequirement};
use regex::Regex;
use semver;
use serde::{Deserialize, Serialize};
use std::fmt;

/// `Requirement` is a wrapper around `semver::Requirement` that adds
/// - Ability to parse go-flavored requirements
///
/// The derived `Default` implementation creates a wildcard version `Requirement`.
#[derive(Debug, Default, PartialEq, Clone)]
pub struct GoRequirement(SemanticVersionRequirement);

#[derive(Debug, Default, PartialEq, Clone)]
pub struct SemanticVersionRequirement(semver::VersionReq);

impl fmt::Display for SemanticVersionRequirement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl VersionRequirement<SemanticVersion> for SemanticVersionRequirement {
    fn satisfies(&self, version: &SemanticVersion) -> bool {
        self.0.matches(&version.0)
    }

    fn parse(input: &str) -> Result<Self, RequirementParseError>
    where
        Self: Sized,
    {
        parse_semver_requirement(input).map(SemanticVersionRequirement)
    }
}

impl VersionRequirement<GoVersion> for GoRequirement {
    fn satisfies<'a>(&self, version: &GoVersion) -> bool {
        self.0.satisfies(&version.0)
    }

    /// Parses a go version requirement `&str` as a `Requirement`
    ///
    /// # Examples
    ///
    /// ```
    /// use heroku_inventory_utils::vrs::VersionRequirement;
    /// let req = heroku_go_utils::vrs::GoRequirement::parse("go1.0").unwrap();
    /// ```
    ///
    /// # Errors
    /// Invalid semver requirement `&str` like ">< 1.0", ".1.0", "!=4", etc.
    /// will return an error.
    fn parse(input: &str) -> Result<Self, RequirementParseError> {
        input
            .strip_prefix("go")
            .map_or(SemanticVersionRequirement::parse(input), |req| {
                SemanticVersionRequirement::parse(format!("={req}").as_str())
            })
            .map(GoRequirement)
    }
}

fn parse_semver_requirement(input: &str) -> Result<semver::VersionReq, RequirementParseError> {
    semver::VersionReq::parse(input).map_err(RequirementParseError)
}

impl fmt::Display for GoRequirement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// `Version` is a wrapper around `semver::Version` that adds
/// - `Deserialize` and `Serialize` traits
/// - Ability to parse go-flavored versions
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct GoVersion(SemanticVersion);

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(try_from = "String", into = "String")]
pub struct SemanticVersion(semver::Version);

impl fmt::Display for SemanticVersion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for SemanticVersion {
    type Error = SemanticVersionParseError;

    fn try_from(val: String) -> Result<Self, Self::Error> {
        SemanticVersion::parse(&val)
    }
}

impl From<SemanticVersion> for String {
    fn from(ver: SemanticVersion) -> Self {
        format!("{ver}")
    }
}

#[derive(thiserror::Error, Debug)]
pub enum SemanticVersionParseError {
    #[error("Couldn't parse semantic version: {0}")]
    SemVer(#[from] semver::Error),
}

impl Version for SemanticVersion {
    type Error = SemanticVersionParseError;

    fn parse(version: &str) -> Result<Self, Self::Error> {
        semver::Version::parse(version)
            .map(SemanticVersion)
            .map_err(SemanticVersionParseError::SemVer)
    }

    fn parse_go(go_version: &str) -> Result<Self, Self::Error> {
        todo!()
    }
}

#[derive(thiserror::Error, Debug)]
pub enum GoVersionParseError {
    #[error("Couldn't parse go version: {0}")]
    SemVer(#[from] semver::Error),
    #[error("Internal buildpack issue parsing go version regex: {0}")]
    Regex(#[from] regex::Error),
    #[error("Couldn't parse version. Unable to capture values from regex.")]
    Captures,
    #[error("Couldn't parse go version: {0}")]
    SemanticVersion(#[from] SemanticVersionParseError),
}

impl Version for GoVersion {
    type Error = GoVersionParseError;
    /// Parses a semver `&str` as a `Version`
    ///
    /// # Examples
    ///
    /// ```
    /// use heroku_inventory_utils::vrs::Version;
    /// let req = heroku_go_utils::vrs::GoVersion::parse("1.14.2").unwrap();
    /// ```
    fn parse(version: &str) -> Result<Self, Self::Error> {
        SemanticVersion::parse(version)
            .map(GoVersion)
            .map_err(GoVersionParseError::SemanticVersion)
    }

    /// Parses a go version `&str` as a `Version`
    ///
    /// # Examples
    ///
    /// ```
    /// use heroku_inventory_utils::vrs::Version;
    /// let req = heroku_go_utils::vrs::GoVersion::parse_go("go1.12").unwrap();
    /// ```
    ///
    /// # Errors
    ///
    /// Invalid go version `&str`s like ".1", "1.*", "abc", etc. will return an error.
    fn parse_go(go_version: &str) -> Result<GoVersion, Self::Error> {
        let stripped_version = go_version.strip_prefix("go").unwrap_or(go_version);

        let caps = Regex::new(r"^(\d+)\.?(\d+)?\.?(\d+)?([a-z][a-z0-9]*)?$")?
            .captures(stripped_version)
            .ok_or(GoVersionParseError::Captures)?;

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

        Self::parse(&composed_version)
    }
}

impl TryFrom<String> for GoVersion {
    type Error = GoVersionParseError;

    fn try_from(val: String) -> Result<Self, Self::Error> {
        GoVersion::parse(&val)
    }
}

impl From<GoVersion> for String {
    fn from(ver: GoVersion) -> Self {
        format!("{ver}")
    }
}

impl fmt::Display for GoVersion {
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
            let actual = GoVersion::parse_go(input).expect("Failed to parse go input version");
            let actual_str = actual.to_string();
            let expected =
                GoVersion::parse(expected_str).expect("Failed to parse go expected version");
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
            let actual = GoRequirement::parse(input)
                .unwrap_or_else(|_| panic!("Failed to parse go input requirement: {input}"));
            let actual_str = actual.to_string();
            let expected = GoRequirement::parse(expected_str)
                .expect("Failed to parse go expected requirement");
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
