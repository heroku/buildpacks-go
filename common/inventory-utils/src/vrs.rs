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