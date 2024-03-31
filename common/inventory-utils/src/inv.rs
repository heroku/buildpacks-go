use crate::vrs::VersionRequirement;
use crate::{checksum::Checksum, vrs::Version};
use core::fmt;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::env::consts;
use std::fs;
use std::hash::Hash;
use std::{fmt::Display, str::FromStr};

/// Represents an inventory of artifacts.
#[derive(Debug, Serialize, Deserialize)]
pub struct Inventory<V>
where
    V: Version,
{
    pub artifacts: Vec<Artifact<V>>,
}

/// Represents a known artifact in the inventory.
#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct Artifact<V>
where
    V: Version,
{
    pub version: V,
    pub os: Os,
    pub arch: Arch,
    pub url: String,
    pub checksum: Checksum,
}

impl<V: Version> Ord for Artifact<V> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (&self.version, &self.arch).cmp(&(&other.version, &other.arch))
    }
}

impl<V: Version> PartialOrd for Artifact<V> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, Ord, PartialOrd)]
#[serde(rename_all = "lowercase")]
pub enum Os {
    Linux,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Ord, PartialOrd)]
#[serde(rename_all = "lowercase")]
pub enum Arch {
    Aarch64,
    X86_64,
}

impl<V: Version> Hash for Artifact<V> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.checksum.value.hash(state);
    }
}

impl<V: Version> Display for Artifact<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({}-{})", self.version, self.os, self.arch)
    }
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
    #[error("Couldn't read artifact inventory.toml: {0}")]
    Io(#[from] std::io::Error),
    #[error("Couldn't parse artifact inventory.toml: {0}")]
    Parse(#[from] toml::de::Error),
}

impl<V> Inventory<V>
where
    V: Version + DeserializeOwned,
{
    /// Read inventory.toml to an `Inventory<V>`.
    ///
    /// # Errors
    ///
    /// Will return an Err if the file is missing, not readable, or if the
    /// file contents is not formatted properly.
    pub fn read(path: &str) -> Result<Self, ReadInventoryError> {
        toml::from_str(&fs::read_to_string(path)?).map_err(ReadInventoryError::Parse)
    }

    /// Find the first artifact from the inventory that satisfies a
    /// `VersionRequirement<V>`.
    #[must_use]
    pub fn resolve<R>(&self, requirement: &R) -> Option<&Artifact<V>>
    where
        R: VersionRequirement<V>,
    {
        match (consts::OS.parse::<Os>(), consts::ARCH.parse::<Arch>()) {
            (Ok(os), Ok(arch)) => self
                .artifacts
                .iter()
                .filter(|artifact| artifact.os == os && artifact.arch == arch)
                .find(|artifact| requirement.satisfies(&artifact.version)),
            (_, _) => None,
        }
    }
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
}
