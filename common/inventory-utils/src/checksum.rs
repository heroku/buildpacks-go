use serde::{Deserialize, Serialize};
use std::{fmt::Display, str::FromStr};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Invalid checksum format: {0}")]
    InvalidFormat(String),
    #[error("Unsupported algorithm: {0}")]
    UnsupportedAlgorithm(String),
    #[error("Invalid checksum length for: {0}")]
    InvalidLength(String),
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Eq, Ord, PartialOrd)]
pub enum Algorithm {
    Sha256,
    Sha512,
}

impl Algorithm {
    fn validate_length(&self, value: &str) -> Result<(), Error> {
        match self {
            Algorithm::Sha256 if value.len() == 64 => Ok(()),
            Algorithm::Sha512 if value.len() == 128 => Ok(()),
            _ => Err(Error::InvalidLength(self.to_string())),
        }
    }
}

impl FromStr for Algorithm {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "sha256" => Ok(Algorithm::Sha256),
            "sha512" => Ok(Algorithm::Sha512),
            _ => Err(Error::UnsupportedAlgorithm(s.to_string())),
        }
    }
}

impl Display for Algorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Algorithm::Sha256 => write!(f, "sha256"),
            Algorithm::Sha512 => write!(f, "sha512"),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Eq, Serialize, Deserialize, Ord, PartialOrd)]
#[serde(try_from = "String", into = "String")]
pub struct Checksum {
    pub(crate) algorithm: Algorithm,
    pub value: String,
}

impl Checksum {
    /// Initialize a new Checksum
    ///
    /// # Errors
    ///
    /// Will return an Error if the checksum value doesn't match the expected
    /// length for the algorithm
    pub fn new(algorithm: Algorithm, value: String) -> Result<Self, Error> {
        algorithm.validate_length(&value)?;
        Ok(Checksum { algorithm, value })
    }
}

impl From<Checksum> for String {
    fn from(value: Checksum) -> Self {
        value.to_string()
    }
}

impl TryFrom<String> for Checksum {
    type Error = Error;

    fn try_from(value: String) -> Result<Self, Error> {
        let parts: Vec<&str> = value.splitn(2, ':').collect();
        if parts.len() == 2 {
            let algorithm: Algorithm = parts[0].parse()?;
            let value = parts[1].to_string();

            Self::new(algorithm, value)
        } else {
            Err(Error::InvalidFormat(value))
        }
    }
}

impl Display for Checksum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.algorithm, self.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checksum_new_valid() {
        let checksum = Checksum::new(
            Algorithm::Sha256,
            "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string(),
        );
        assert!(checksum.is_ok());
    }

    #[test]
    fn test_checksum_new_invalid_length() {
        let checksum = Checksum::new(Algorithm::Sha256, "foo".to_string());
        assert!(checksum.is_err());
    }

    #[test]
    fn test_checksum_parse_and_validate_sha256() {
        let checksum: Result<Checksum, _> = Checksum::try_from(
            "sha256:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string(),
        );

        assert!(checksum.is_ok());
        assert_eq!(Algorithm::Sha256, checksum.unwrap().algorithm);
    }

    #[test]
    fn test_checksum_parse_and_validate_sha512() {
        let checksum: Result<Checksum, _> = Checksum::try_from(
            "sha512:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string()
        );

        assert!(checksum.is_ok());
        assert_eq!(Algorithm::Sha512, checksum.unwrap().algorithm);
    }

    #[test]
    fn test_checksum_serialization() {
        assert_eq!(
            "sha256:foo",
            Checksum {
                algorithm: Algorithm::Sha256,
                value: "foo".to_string(),
            }
            .to_string()
        );
    }

    #[test]
    fn test_invalid_checksum_length() {
        assert!(matches!(
            Checksum::try_from("sha256:abc".to_string()),
            Err(Error::InvalidLength(..))
        ));
    }

    #[test]
    fn test_unsupported_algorithm() {
        assert!(matches!(
            Checksum::try_from("md5:abcdef1234567890abcdef1234567890".to_string()),
            Err(Error::UnsupportedAlgorithm(..))
        ));
    }
}
