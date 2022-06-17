use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use std::io::Read;
use tar::Archive;

#[derive(thiserror::Error, Debug)]
pub enum DistError {
    // Boxed to prevent `large_enum_variant` errors since `ureq::Error` is massive.
    #[error("HTTP error while downloading go distribution: {0}")]
    Http(#[from] Box<ureq::Error>),

    #[error("IO error while downloading go distribution: {0}")]
    Io(#[from] std::io::Error),

    #[error("Failed to validate checksum for go distribution")]
    Checksum,

    #[error("Go distribution contents missing leading directory")]
    Prefix,
}

pub fn download_validate_extract(
    uri: impl AsRef<str>,
    sha256: impl AsRef<str>,
    dest_dir: impl AsRef<std::path::Path>,
) -> Result<(), DistError> {
    let expected_digest = sha256.as_ref();
    let destination = dest_dir.as_ref();
    let body = ureq::get(uri.as_ref())
        .call()
        .map_err(Box::new)?
        .into_reader();
    let mut archive = Archive::new(GzDecoder::new(HashingReader::new(body, Sha256::new())));
    for entry in archive.entries()? {
        let mut file = entry?;
        let path = destination.join(
            file.path()?
                .strip_prefix("go/")
                .map_err(|_| DistError::Prefix)?,
        );
        if ["bin", "pkg", "lib", "src", "LICENSE"]
            .iter()
            .any(|prefix| path.starts_with(destination.join(prefix)))
        {
            file.unpack(&path)?;
        }
    }
    let actual_digest = format!(
        "{:x}",
        archive.into_inner().into_inner().hasher().finalize()
    );
    if expected_digest != actual_digest {
        return Err(DistError::Checksum);
    }
    Ok(())
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
    fn test_download_validate_extract() {
        let dest = tempfile::tempdir().expect("Couldn't create test tmpdir");
        download_validate_extract(
            "https://go.dev/dl/go1.12.linux-amd64.tar.gz",
            "750a07fef8579ae4839458701f4df690e0b20b8bcce33b437e4df89c451b6f13",
            dest.path(),
        )
        .expect("Expected to download validate and extract");
        for file in std::fs::read_dir(dest.path().join("bin")).unwrap() {
            println!("{}", file.unwrap().path().display());
        }

        let go_path = dest.path().join("bin").join("go");
        assert!(go_path.exists());
    }
}
