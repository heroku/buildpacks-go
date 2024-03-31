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

#[cfg(test)]
mod tests {
    use crate::{
        checksum::{Algorithm, Checksum},
        inv::{Arch, Artifact, Os},
    };

    use super::*;
    use std::hash::{BuildHasher, RandomState};

    fn create_artifact() -> Artifact<SemanticVersion> {
        Artifact::<SemanticVersion> {
            version: SemanticVersion::parse("1.7.2").unwrap(),
            os: Os::Linux,
            arch: Arch::X86_64,
            url: String::from("foo"),
            checksum: Checksum::new(
                Algorithm::Sha256,
                "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string(),
            )
            .unwrap(),
        }
    }

    #[test]
    fn test_artifact_display_format() {
        let artifact = create_artifact();

        assert_eq!("1.7.2 (linux-x86_64)", artifact.to_string());
    }

    #[test]
    fn test_artifact_hash_implementation() {
        let artifact = create_artifact();

        let state = RandomState::new();
        assert_eq!(
            state.hash_one(&artifact.checksum.value),
            state.hash_one(&artifact)
        );
    }

    #[test]
    fn test_artifact_serialization() {
        let artifact = create_artifact();
        let serialized = toml::to_string(&artifact).unwrap();
        assert!(serialized
            .contains("sha256:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"));
        assert_eq!(
            artifact,
            toml::from_str::<Artifact<SemanticVersion>>(&serialized).unwrap()
        );
    }
}
