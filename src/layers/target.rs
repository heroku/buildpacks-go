use crate::{GoBuildpack, GoBuildpackError};
use libcnb::build::BuildContext;
use libcnb::data::layer_content_metadata::LayerTypes;
use libcnb::generic::GenericMetadata;
use libcnb::layer::{Layer, LayerResult, LayerResultBuilder};
use std::fs;
use std::io;
use std::path::Path;
use thiserror::Error;

/// An empty, run-only, layer for compiled Go app binaries.
pub struct TargetLayer {}

#[derive(Error, Debug)]
#[error("Couldn't write to target layer: {0}")]
pub struct TargetLayerError(io::Error);

impl Layer for TargetLayer {
    type Buildpack = GoBuildpack;
    type Metadata = GenericMetadata;

    fn types(&self) -> LayerTypes {
        LayerTypes {
            build: false,
            launch: true,
            cache: false,
        }
    }

    // This layer creates a `bin` directory, which is the target for `go build`
    // later.
    fn create(
        &self,
        _context: &BuildContext<Self::Buildpack>,
        layer_path: &Path,
    ) -> Result<LayerResult<Self::Metadata>, GoBuildpackError> {
        fs::create_dir(layer_path.join("bin")).map_err(TargetLayerError)?;
        LayerResultBuilder::new(GenericMetadata::default()).build()
    }
}
