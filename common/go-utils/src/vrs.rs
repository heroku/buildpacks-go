use libherokubuildpack::inventory::version::VersionRequirement;
use regex::Regex;
use semver;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

impl VersionRequirement<GoVersion> for semver::VersionReq {
    fn satisfies(&self, version: &GoVersion) -> bool {
        self.matches(&version.semver)
    }
}

/// Parses a `semver::VersionReq` from a go-flavored requirement `&str`
///
/// # Examples
///
/// ```
/// use heroku_go_utils::vrs::parse_go_version_requirement;
/// let req = parse_go_version_requirement("go1.0").unwrap();
/// ```
///
/// # Errors
/// Invalid semver requirement `&str` like ">< 1.0", ".1.0", "!=4", etc.
/// will return an error.
pub fn parse_go_version_requirement(input: &str) -> Result<semver::VersionReq, semver::Error> {
    semver::VersionReq::parse(
        &input
            .strip_prefix("go")
            .map_or_else(|| input.to_string(), |v| format!("={v}")),
    )
}

/// `GoVersion` is a wrapper around a `semver::Version` that can be
///  parsed from go-flavored version strings
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[serde(try_from = "String", into = "String")]
pub struct GoVersion {
    pub value: String,
    #[serde(skip)]
    pub semver: semver::Version,
}

impl GoVersion {
    /// Get the corresponding Go major release. Go identifies major releases
    /// as increments to the second identifier (e.g.: 1.16.0, 1.22.0). In
    /// semver this corresponds to a change to major and/or minor identifiers.
    #[must_use]
    pub fn major_release_version(&self) -> GoVersion {
        let go_major_release = semver::Version::new(self.semver.major, self.semver.minor, 0);

        GoVersion {
            value: go_major_release.to_string(),
            semver: go_major_release,
        }
    }
}

impl Display for GoVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl From<GoVersion> for String {
    fn from(version: GoVersion) -> Self {
        version.value
    }
}

impl Ord for GoVersion {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.semver.cmp(&other.semver)
    }
}

impl PartialOrd for GoVersion {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
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
}

impl TryFrom<String> for GoVersion {
    type Error = GoVersionParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let stripped_version = value.strip_prefix("go").unwrap_or(&value);

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
        let semver = semver::Version::parse(&composed_version)?;
        Ok(GoVersion { value, semver })
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
            let actual =
                GoVersion::try_from(input.to_string()).expect("Failed to parse go input version");
            let expected = semver::Version::parse(expected_str).unwrap();

            assert_eq!(
                expected, actual.semver,
                "Expected {input} to parse as {expected} but got {}.",
                actual.semver
            );
            assert_eq!(
                input,
                actual.to_string(),
                "Expected Go parsed from {input} to be displayed as {actual}"
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
            let actual = parse_go_version_requirement(input).unwrap();
            let expected = parse_go_version_requirement(expected_str).unwrap();

            assert_eq!(
                expected, actual,
                "Expected {input} to parse as {expected} but got {actual}."
            );

            let actual_str = actual.to_string();
            assert_eq!(
                expected_str, actual_str,
                "Expected {input} to parse as {expected_str} but got {actual_str}"
            );
        }
    }

    #[test]
    fn test_version_ordering() {
        let examples = [("1.20.1", "1.2.1"), ("1.20.0", "1.3.0")];
        for (version, other_version) in examples {
            assert_eq!(
                std::cmp::Ordering::Greater,
                GoVersion::try_from(String::from(version))
                    .unwrap()
                    .cmp(&GoVersion::try_from(String::from(other_version)).unwrap())
            );
        }
    }
}
