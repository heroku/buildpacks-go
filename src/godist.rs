use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use std::{io::Read, path::StripPrefixError};
use tar::Archive;

#[derive(thiserror::Error, Debug)]
pub enum DistError {
    // Boxed to prevent `large_enum_variant` errors since `ureq::Error` is massive.
    #[error("HTTP error while downloading go distribution: {0}")]
    Http(#[from] Box<ureq::Error>),

    #[error("Error reading archive entries: {0}")]
    Entries(std::io::Error),

    #[error("Error reading archive entry: {0}")]
    Entry(std::io::Error),

    #[error("Error reading archive file path: {0}")]
    Path(std::io::Error),

    #[error("Failed to validate checksum; expected {0}, but found {1}")]
    Checksum(String, String),

    #[error("Error writing archive entry: {0}")]
    Unpack(std::io::Error),

    #[error("Error stripping archive entry prefix: {0}")]
    Prefix(StripPrefixError),
}

/// Fetches a tarball from a url, strips component paths, filters path prefixes,
/// and verifies a sha256 checksum, without writing temporary files or
/// reading the tarball fully into memory.
pub fn fetch_strip_filter_extract_verify<'a>(
    uri: impl AsRef<str>,
    strip_prefix: impl AsRef<str>,
    filter_prefixes: impl Iterator<Item = &'a str>,
    dest_dir: impl AsRef<std::path::Path>,
    sha256: impl AsRef<str>,
) -> Result<(), DistError> {
    let expected_digest = sha256.as_ref();
    let destination = dest_dir.as_ref();
    let body = ureq::get(uri.as_ref())
        .call()
        .map_err(Box::new)?
        .into_reader();
    let mut archive = Archive::new(GzDecoder::new(HashingReader::new(body, Sha256::new())));
    let filters: Vec<&str> = filter_prefixes.into_iter().collect();
    for entry in archive.entries().map_err(DistError::Entries)? {
        let mut file = entry.map_err(DistError::Entry)?;
        let path = destination.join(
            file.path()
                .map_err(DistError::Path)?
                .strip_prefix(strip_prefix.as_ref())
                .map_err(DistError::Prefix)?,
        );
        if filters
            .iter()
            .any(|prefix| path.starts_with(destination.join(prefix)))
        {
            file.unpack(&path).map_err(DistError::Unpack)?;
        }
    }
    let actual_digest = format!(
        "{:x}",
        archive.into_inner().into_inner().hasher().finalize()
    );
    (expected_digest == actual_digest)
        .then(|| ())
        .ok_or_else(|| DistError::Checksum(expected_digest.to_string(), actual_digest))
}

struct HashingReader<R: Read, H: sha2::Digest> {
    r: R,
    h: H,
}

impl<R: Read, H: sha2::Digest> HashingReader<R, H> {
    pub fn new(reader: R, hasher: H) -> HashingReader<R, H> {
        HashingReader {
            r: reader,
            h: hasher,
        }
    }
    pub fn hasher(self) -> H {
        self.h
    }
}

impl<R: Read, H: sha2::Digest> Read for HashingReader<R, H> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.r.read(buf)?;
        self.h.update(&buf[..n]);
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fetch_strip_filter_extract_verify() {
        let dest = tempfile::tempdir().expect("Couldn't create test tmpdir");
        fetch_strip_filter_extract_verify(
            "https://mirrors.edge.kernel.org/pub/software/scm/git/git-0.01.tar.gz",
            "git-0.01",
            ["README"].into_iter(),
            dest.path(),
            "9bdf8a4198b269c5cbe4263b1f581aae885170a6cb93339a2033cb468e57dcd3",
        )
        .expect("Expected to fetch, strip, filter, extract, and verify");

        let target_path = dest.path().join("README");
        assert!(target_path.exists());
    }
}
