use hex::FromHexError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256, Sha512};
use std::marker::PhantomData;

#[derive(Debug, Clone, Eq)]
pub struct Checksum<D> {
    pub value: Vec<u8>,
    digest: PhantomData<D>,
}

impl<D> PartialEq for Checksum<D> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Invalid checksum size for: {0}")]
    InvalidSize(String),
    #[error("Invalid input string")]
    HexError(#[from] FromHexError),
}

impl<D> TryFrom<String> for Checksum<D>
where
    D: ChecksumSize,
{
    type Error = Error;

    fn try_from(input: String) -> Result<Self, Self::Error> {
        let value: Vec<u8> = hex::decode(input.clone())?;
        if value.len() == D::checksum_size() {
            Ok(Checksum {
                value,
                digest: PhantomData,
            })
        } else {
            Err(Error::InvalidSize(input))
        }
    }
}

pub trait Name {
    fn name() -> String;
}

impl Name for Sha256 {
    fn name() -> String {
        String::from("sha256")
    }
}

impl Name for Sha512 {
    fn name() -> String {
        String::from("sha512")
    }
}

#[allow(clippy::module_name_repetitions)]
pub trait ChecksumSize {
    fn checksum_size() -> usize;
}

impl ChecksumSize for Sha256 {
    fn checksum_size() -> usize {
        Self::output_size()
    }
}

impl ChecksumSize for Sha512 {
    fn checksum_size() -> usize {
        Self::output_size()
    }
}

impl<D> Serialize for Checksum<D>
where
    D: Name,
{
    fn serialize<T>(&self, serializer: T) -> Result<T::Ok, T::Error>
    where
        T: serde::Serializer,
    {
        serializer.serialize_str(&format!("{}:{}", D::name(), hex::encode(&self.value)))
    }
}

impl<'de, D> Deserialize<'de> for Checksum<D>
where
    D: Name,
{
    fn deserialize<T>(deserializer: T) -> Result<Self, T::Error>
    where
        T: serde::Deserializer<'de>,
    {
        use serde::de::Error;
        String::deserialize(deserializer)?
            .strip_prefix(&format!("{}:", D::name()))
            .ok_or_else(|| T::Error::custom("checksum prefix is invalid"))
            .map(|value| hex::decode(value).map_err(T::Error::custom))?
            .map(|value| Checksum::<_> {
                value,
                digest: PhantomData,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_test::{assert_de_tokens_error, assert_tokens, Token};
    use sha2::Sha512;

    impl Name for String {
        fn name() -> String {
            String::from("foo")
        }
    }

    impl ChecksumSize for String {
        fn checksum_size() -> usize {
            2
        }
    }

    #[test]
    fn test_checksum_serialization() {
        assert_tokens(
            &Checksum::<String>::try_from("abcd".to_string()).unwrap(),
            &[Token::BorrowedStr("foo:abcd")],
        );
    }

    #[test]
    fn test_invalid_checksum_deserialization() {
        assert_de_tokens_error::<Checksum<String>>(
            &[Token::BorrowedStr("baz:bar")],
            "checksum prefix is invalid",
        );
    }

    #[test]
    fn test_digest_names() {
        assert_eq!("sha256", Sha256::name());
        assert_eq!("sha512", Sha512::name());
    }

    #[test]
    fn test_checksum_sizes() {
        assert_eq!(32, Sha256::checksum_size());
        assert_eq!(64, Sha512::checksum_size());
    }

    #[test]
    fn test_invalid_checksum_size() {
        assert!(matches!(
            Checksum::<Sha256>::try_from("123456".to_string()),
            Err(Error::InvalidSize(..))
        ));
    }

    #[test]
    fn test_invalid_hex_input() {
        assert!(matches!(
            Checksum::<Sha256>::try_from("quux".to_string()),
            Err(Error::HexError(..))
        ));
    }

    #[test]
    fn test_sha256_checksum_parse_and_serialize() {
        let result: Result<Checksum<Sha256>, _> = Checksum::try_from(
            "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string(),
        );

        assert!(result.is_ok());
        assert_tokens(
            &result.unwrap(),
            &[Token::BorrowedStr(
                "sha256:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
            )],
        );
    }

    #[test]
    fn test_sha512_checksum_parse_and_serialize() {
        let result: Result<Checksum<Sha512>, _> = Checksum::try_from(
            "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string(),
        );

        assert!(result.is_ok());
        assert_tokens(
            &result.unwrap(),
            &[Token::BorrowedStr(
                "sha512:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
            )],
        );
    }
}
