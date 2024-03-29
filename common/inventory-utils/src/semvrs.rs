use crate::vrs::{RequirementParseError, Version, VersionRequirement};
use core::fmt;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Ord, PartialOrd)]
#[serde(try_from = "String", into = "String")]
pub struct SemanticVersion(semver::Version);

impl Display for SemanticVersion {
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

    /// Parses a semver `&str` as a `SemanticVersion`
    ///
    /// # Examples
    ///
    /// ```
    /// use heroku_inventory_utils::vrs::Version;
    /// let req = heroku_inventory_utils::semvrs::SemanticVersion::parse("1.14.2").unwrap();
    /// ```
    fn parse(version: &str) -> Result<Self, Self::Error> {
        semver::Version::parse(version)
            .map(SemanticVersion)
            .map_err(SemanticVersionParseError::SemVer)
    }
}

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
        semver::VersionReq::parse(input)
            .map_err(RequirementParseError)
            .map(SemanticVersionRequirement)
    }
}
