use crate::inv::{Version, VersionRequirement};

impl VersionRequirement<semver::Version> for semver::VersionReq {
    fn satisfies(&self, version: &semver::Version) -> bool {
        self.matches(version)
    }
}

impl Version for semver::Version {}
