use crate::inv::VersionRequirement;

impl VersionRequirement<semver::Version> for semver::VersionReq {
    fn satisfies(&self, version: &semver::Version) -> bool {
        self.matches(version)
    }
}
