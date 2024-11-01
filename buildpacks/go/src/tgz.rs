use flate2::read::GzDecoder;
use libherokubuildpack::inventory::artifact::Artifact;
use sha2::{
    digest::{generic_array::GenericArray, OutputSizeUser},
    Digest,
};
use std::{fs, io::Read, path::StripPrefixError};
use tar::Archive;

#[derive(thiserror::Error, Debug)]
pub(crate) enum Error {
    #[error("HTTP error while fetching archive: {0}")]
    Http(#[from] Box<ureq::Error>),

    #[error("Error reading archive entries: {0}")]
    Entries(std::io::Error),

    #[error("Error reading archive entry: {0}")]
    Entry(std::io::Error),

    #[error("Error reading archive file path: {0}")]
    Path(std::io::Error),

    #[error("Failed to validate archive checksum; expected {0}, but found {1}")]
    Checksum(String, String),

    #[error("Error creating archive directory: {0}")]
    Directory(std::io::Error),

    #[error("Error writing archive entry: {0}")]
    Unpack(std::io::Error),

    #[error("Error stripping archive entry prefix: {0}")]
    Prefix(StripPrefixError),
}

/// Fetches a tarball from the artifact url, strips component paths, filters path
/// prefixes, extracts files to a directory, and verifies the artifact checksum. Care
/// is taken not to write temporary files or read the entire contents into memory.
/// In an error scenario, any archive contents already extracted will not be removed.
///
/// # Errors
///
/// See `Error` for an enumeration of error scenarios.
pub(crate) fn fetch_strip_filter_extract_verify<'a, D: Digest, V>(
    artifact: &Artifact<V, D, Option<()>>,
    strip_prefix: impl AsRef<str>,
    filter_prefixes: impl Iterator<Item = &'a str>,
    dest_dir: impl AsRef<std::path::Path>,
) -> Result<(), Error> {
    let destination = dest_dir.as_ref();
    let body = ureq::get(artifact.url.as_ref())
        .call()
        .map_err(Box::new)?
        .into_reader();

    let mut archive = Archive::new(GzDecoder::new(DigestingReader::new(body, D::new())));
    let filters: Vec<&str> = filter_prefixes.into_iter().collect();
    for entry in archive.entries().map_err(Error::Entries)? {
        let mut file = entry.map_err(Error::Entry)?;
        let path = destination.join(
            file.path()
                .map_err(Error::Path)?
                .strip_prefix(strip_prefix.as_ref())
                .map_err(Error::Prefix)?,
        );
        if filters
            .iter()
            .any(|prefix| path.starts_with(destination.join(prefix)))
        {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(Error::Directory)?;
            }
            file.unpack(&path).map_err(Error::Unpack)?;
        }
    }
    let actual_digest = archive.into_inner().into_inner().finalize();
    (actual_digest.to_vec() == artifact.checksum.value)
        .then_some(())
        .ok_or_else(|| {
            Error::Checksum(
                hex::encode(artifact.checksum.value.clone()),
                hex::encode(actual_digest),
            )
        })
}

struct DigestingReader<R: Read, H: sha2::Digest> {
    r: R,
    h: H,
}

impl<R: Read, H: sha2::Digest> DigestingReader<R, H> {
    pub(crate) fn new(reader: R, hasher: H) -> DigestingReader<R, H> {
        DigestingReader {
            r: reader,
            h: hasher,
        }
    }
    pub(crate) fn finalize(self) -> GenericArray<u8, <H as OutputSizeUser>::OutputSize> {
        self.h.finalize()
    }
}

impl<R: Read, H: sha2::Digest> Read for DigestingReader<R, H> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.r.read(buf)?;
        self.h.update(&buf[..n]);
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use libherokubuildpack::inventory::{
        artifact::{Arch, Os},
        checksum::Checksum,
    };
    use sha2::Sha256;

    use super::*;

    #[test]
    fn test_fetch_strip_filter_extract_verify() {
        let artifact = Artifact::<String, Sha256, Option<()>> {
            version: "0.0.1".to_string(),
            os: Os::Linux,
            arch: Arch::Amd64,
            url: "https://mirrors.edge.kernel.org/pub/software/scm/git/git-0.01.tar.gz".to_string(),
            checksum: "sha256:9bdf8a4198b269c5cbe4263b1f581aae885170a6cb93339a2033cb468e57dcd3"
                .to_string()
                .parse::<Checksum<Sha256>>()
                .unwrap(),
            metadata: None,
        };
        let dest = tempfile::tempdir().expect("Couldn't create test tmpdir");

        fetch_strip_filter_extract_verify(
            &artifact,
            "git-0.01",
            ["README"].into_iter(),
            dest.path(),
        )
        .expect("Expected to fetch, strip, filter, extract, and verify");

        let target_path = dest.path().join("README");
        assert!(target_path.exists());
    }
}
