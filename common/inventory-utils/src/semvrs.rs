use crate::vrs::VersionRequirement;

impl VersionRequirement<semver::Version> for semver::VersionReq {
    fn satisfies(&self, version: &semver::Version) -> bool {
        self.matches(version)
    }
}
