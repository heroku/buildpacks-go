use heroku_inventory_utils::{
    semvrs::{SemanticVersion, SemanticVersionParseError, SemanticVersionRequirement},
    vrs::{RequirementParseError, Version, VersionRequirement},
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fmt;

/// `Requirement` is a wrapper around `semver::Requirement` that adds
/// - Ability to parse go-flavored requirements
///
/// The derived `Default` implementation creates a wildcard version `Requirement`.
#[derive(Debug, Default, PartialEq, Clone)]
pub struct GoRequirement(SemanticVersionRequirement);

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

impl fmt::Display for GoRequirement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// `GoVersion` is a wrapper around `SemanticVersion` that adds
///  ability to parse go-flavored versions
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct GoVersion(SemanticVersion);

#[derive(thiserror::Error, Debug)]
pub enum GoVersionParseError {
    #[error("Internal buildpack issue parsing go version regex: {0}")]
    Regex(#[from] regex::Error),
    #[error("Couldn't parse version. Unable to capture values from regex.")]
    Captures,
    #[error("Couldn't parse go version: {0}")]
    SemanticVersion(#[from] SemanticVersionParseError),
}

impl Version for GoVersion {
    type Error = GoVersionParseError;

    /// Parses a go version `&str` as a `Version`
    ///
    /// # Examples
    ///
    /// ```
    /// use heroku_inventory_utils::vrs::Version;
    /// let req = heroku_go_utils::vrs::GoVersion::parse("go1.12").unwrap();
    /// ```
    ///
    /// # Errors
    ///
    /// Invalid go version `&str`s like ".1", "1.*", "abc", etc. will return an error.
    fn parse(version: &str) -> Result<Self, Self::Error> {
        let stripped_version = version.strip_prefix("go").unwrap_or(version);

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

        SemanticVersion::parse(&composed_version)
            .map(GoVersion)
            .map_err(GoVersionParseError::SemanticVersion)
    }
}

impl TryFrom<String> for GoVersion {
    type Error = GoVersionParseError;

    fn try_from(val: String) -> Result<Self, Self::Error> {
        GoVersion::parse(&val)
    }
}

impl fmt::Display for GoVersion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Go {}", self.0)
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
            let actual = GoVersion::parse(input).expect("Failed to parse go input version");
            let actual_str = actual.to_string();
            let expected = GoVersion(
                SemanticVersion::parse(expected_str).expect("Failed to parse go expected version"),
            );
            assert_eq!(
                expected, actual,
                "Expected {input} to parse as {expected} but got {actual}."
            );
            assert_eq!(
                format!("Go {expected_str}"),
                actual_str,
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
