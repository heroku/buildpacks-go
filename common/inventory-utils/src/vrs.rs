pub trait Version: Sized {
    type Error;

    /// # Errors
    ///
    /// Invalid Version `&str`s for the implementation will return an error.
    fn parse(version: &str) -> Result<Self, Self::Error>;

    /// # Errors
    ///
    /// Invalid go version `&str`s like ".1", "1.*", "abc", etc. will return an error.
    /// This trait fn will be refactored.
    fn parse_go(go_version: &str) -> Result<Self, Self::Error>;
}

pub trait VersionRequirement<T> {
    fn satisfies(&self, version: &T) -> bool;

    /// Parses a &str as a `VersionRequirement<Version>`.

    /// # Errors
    /// Invalid semver requirement `&str` like ">< 1.0", ".1.0", "!=4", etc.
    /// will return an error.
    fn parse(input: &str) -> Result<Self, RequirementParseError>
    where
        Self: Sized;
}

#[derive(thiserror::Error, Debug)]
#[error("Couldn't parse semantic version requirement: {0}")]
pub struct RequirementParseError(#[from] pub semver::Error);
