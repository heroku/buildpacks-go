use crate::checksum::Checksum;
use crate::checksum::Name;
use core::fmt;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fs;
use std::hash::Hash;
use std::{fmt::Display, str::FromStr};

/// Represents an inventory of artifacts.
#[derive(Debug, Serialize, Deserialize)]
pub struct Inventory<V, D> {
    #[serde(bound = "V: Serialize + DeserializeOwned, D: Name")]
    pub artifacts: Vec<Artifact<V, D>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Artifact<V, D> {
    #[serde(bound = "V: Serialize + DeserializeOwned")]
    pub version: V,
    pub os: Os,
    pub arch: Arch,
    pub url: String,
    #[serde(bound = "D: Name")]
    pub checksum: Checksum<D>,
}

impl<V, D> PartialEq for Artifact<V, D>
where
    V: Eq,
{
    fn eq(&self, other: &Self) -> bool {
        self.version == other.version
            && self.os == other.os
            && self.arch == other.arch
            && self.url == other.url
            && self.checksum == other.checksum
    }
}

impl<V, D> Eq for Artifact<V, D> where V: Eq {}

impl<V, D> Hash for Artifact<V, D> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.checksum.value.hash(state);
    }
}

impl<V: Display, D> Display for Artifact<V, D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({}-{})", self.version, self.os, self.arch)
    }
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Os {
    Darwin,
    Linux,
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Arch {
    Amd64,
    Arm64,
}

impl Display for Os {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Os::Darwin => write!(f, "darwin"),
            Os::Linux => write!(f, "linux"),
        }
    }
}

#[derive(thiserror::Error, Debug)]
#[error("OS is not supported: {0}")]
pub struct UnsupportedOsError(String);

impl FromStr for Os {
    type Err = UnsupportedOsError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "linux" => Ok(Os::Linux),
            "darwin" | "osx" => Ok(Os::Darwin),
            _ => Err(UnsupportedOsError(s.to_string())),
        }
    }
}

impl Display for Arch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Arch::Amd64 => write!(f, "amd64"),
            Arch::Arm64 => write!(f, "arm64"),
        }
    }
}

#[derive(thiserror::Error, Debug)]
#[error("Arch is not supported: {0}")]
pub struct UnsupportedArchError(String);

impl FromStr for Arch {
    type Err = UnsupportedArchError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "amd64" | "x86_64" => Ok(Arch::Amd64),
            "arm64" | "aarch64" => Ok(Arch::Arm64),
            _ => Err(UnsupportedArchError(s.to_string())),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ReadInventoryError {
    #[error("Couldn't read inventory file: {0}")]
    Io(#[from] std::io::Error),
    #[error("Couldn't parse inventory toml: {0}")]
    Parse(#[from] toml::de::Error),
}

/// Reads a TOML-formatted file to an `Inventory<V, D>`.
///
/// # Errors
///
/// Will return an Err if the file is missing, not readable, or if the
/// file contents is not formatted properly.
pub fn read_inventory_file<V, D>(path: &str) -> Result<Inventory<V, D>, ReadInventoryError>
where
    V: Serialize + DeserializeOwned,
    D: Name,
{
    toml::from_str(&fs::read_to_string(path)?).map_err(ReadInventoryError::Parse)
}

pub trait VersionRequirement<V> {
    fn satisfies(&self, version: &V) -> bool;
}

/// Find the first artifact that satisfies a `VersionRequirement<V>` for
/// the specified OS and arch.
pub fn resolve<'a, V, D, R>(
    artifacts: &'a [Artifact<V, D>],
    os: Os,
    arch: Arch,
    requirement: &'a R,
) -> Option<&'a Artifact<V, D>>
where
    R: VersionRequirement<V>,
{
    artifacts
        .iter()
        .filter(|artifact| artifact.os == os && artifact.arch == arch)
        .find(|artifact| requirement.satisfies(&artifact.version))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_arch_display_format() {
        let archs = [(Arch::Amd64, "amd64"), (Arch::Arm64, "arm64")];

        for (input, expected) in archs {
            assert_eq!(expected, input.to_string());
        }
    }

    #[test]
    fn test_arch_parsing() {
        let archs = [
            ("amd64", Arch::Amd64),
            ("arm64", Arch::Arm64),
            ("x86_64", Arch::Amd64),
            ("aarch64", Arch::Arm64),
        ];
        for (input, expected) in archs {
            assert_eq!(expected, input.parse::<Arch>().unwrap());
        }

        assert!(matches!(
            "foo".parse::<Arch>().unwrap_err(),
            UnsupportedArchError(..)
        ));
    }

    #[test]
    fn test_os_display_format() {
        assert_eq!("linux", Os::Linux.to_string());
    }

    #[test]
    fn test_os_parsing() {
        assert_eq!(Os::Linux, "linux".parse::<Os>().unwrap());
        assert_eq!(Os::Darwin, "darwin".parse::<Os>().unwrap());
        assert_eq!(Os::Darwin, "osx".parse::<Os>().unwrap());

        assert!(matches!(
            "foo".parse::<Os>().unwrap_err(),
            UnsupportedOsError(..)
        ));
    }

    #[test]
    fn test_artifact_display() {
        assert_eq!(
            "foo (linux-arm64)",
            create_artifact("foo", Os::Linux, Arch::Arm64).to_string()
        );
    }

    impl VersionRequirement<String> for String {
        fn satisfies(&self, version: &String) -> bool {
            self == version
        }
    }

    #[test]
    fn test_matching_artifact_resolution() {
        assert_eq!(
            "foo",
            &resolve(
                &[create_artifact("foo", Os::Linux, Arch::Arm64)],
                Os::Linux,
                Arch::Arm64,
                &String::from("foo")
            )
            .expect("should resolve matching artifact")
            .version,
        );
    }

    #[test]
    fn test_dont_resolve_artifact_with_wrong_arch() {
        assert!(resolve(
            &[create_artifact("foo", Os::Linux, Arch::Arm64)],
            Os::Linux,
            Arch::Amd64,
            &String::from("foo")
        )
        .is_none());
    }

    #[test]
    fn test_dont_resolve_artifact_with_wrong_version() {
        assert!(resolve(
            &[create_artifact("foo", Os::Linux, Arch::Arm64)],
            Os::Linux,
            Arch::Arm64,
            &String::from("bar")
        )
        .is_none());
    }

    fn create_artifact(version: &str, os: Os, arch: Arch) -> Artifact<String, String> {
        Artifact::<String, String> {
            version: String::from(version),
            os,
            arch,
            url: "https://example.com".to_string(),
            checksum: Checksum::try_from("aaaa".to_string()).unwrap(),
        }
    }
}
