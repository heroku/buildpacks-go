use crate::checksum::Name;
use crate::vrs::VersionRequirement;
use crate::{checksum::Checksum, vrs::Version};
use core::fmt;
use serde::{Deserialize, Serialize};
use std::fs;
use std::hash::Hash;
use std::{fmt::Display, str::FromStr};

/// Represents an inventory of artifacts.
#[derive(Debug, Serialize, Deserialize)]
pub struct Inventory<V, D> {
    #[serde(bound = "V: Version, D: Name")]
    pub artifacts: Vec<Artifact<V, D>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Artifact<V, D> {
    #[serde(bound = "V: Version")]
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
    Linux,
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Arch {
    X86_64,
    Aarch64,
}

impl Display for Os {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
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
            _ => Err(UnsupportedOsError(s.to_string())),
        }
    }
}

impl Display for Arch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Arch::X86_64 => write!(f, "x86_64"),
            Arch::Aarch64 => write!(f, "aarch64"),
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
            "amd64" | "x86_64" => Ok(Arch::X86_64),
            "arm64" | "aarch64" => Ok(Arch::Aarch64),
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

impl<V, D> Inventory<V, D>
where
    V: Version,
    D: Name,
{
    /// Read a TOML-formatted file to an `Inventory<V, D>`.
    ///
    /// # Errors
    ///
    /// Will return an Err if the file is missing, not readable, or if the
    /// file contents is not formatted properly.
    pub fn read(path: &str) -> Result<Self, ReadInventoryError> {
        toml::from_str(&fs::read_to_string(path)?).map_err(ReadInventoryError::Parse)
    }
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
        let archs = [(Arch::X86_64, "x86_64"), (Arch::Aarch64, "aarch64")];

        for (input, expected) in archs {
            assert_eq!(expected, input.to_string());
        }
    }

    #[test]
    fn test_arch_parsing() {
        let archs = [
            ("amd64", Arch::X86_64),
            ("arm64", Arch::Aarch64),
            ("x86_64", Arch::X86_64),
            ("aarch64", Arch::Aarch64),
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

        assert!(matches!(
            "foo".parse::<Os>().unwrap_err(),
            UnsupportedOsError(..)
        ));
    }

    #[test]
    fn test_artifact_display() {
        assert_eq!(
            "foo (linux-aarch64)",
            create_artifact("foo", Os::Linux, Arch::Aarch64).to_string()
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
                &[create_artifact("foo", Os::Linux, Arch::Aarch64)],
                Os::Linux,
                Arch::Aarch64,
                &String::from("foo")
            )
            .expect("should resolve matching artifact")
            .version,
        );
    }

    #[test]
    fn test_dont_resolve_artifact_with_wrong_arch() {
        assert!(resolve(
            &[create_artifact("foo", Os::Linux, Arch::Aarch64)],
            Os::Linux,
            Arch::X86_64,
            &String::from("foo")
        )
        .is_none());
    }

    #[test]
    fn test_dont_resolve_artifact_with_wrong_version() {
        assert!(resolve(
            &[create_artifact("foo", Os::Linux, Arch::Aarch64)],
            Os::Linux,
            Arch::Aarch64,
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
