use crate::{GoBuildpack, GoBuildpackError};
use libcnb::build::BuildContext;
use libcnb::data::layer_content_metadata::LayerTypes;
use libcnb::generic::GenericMetadata;
use libcnb::layer::{Layer, LayerResult, LayerResultBuilder};
use libcnb::layer_env::{LayerEnv, Scope};
use std::fs;
use std::io;
use std::path::Path;
use thiserror::Error;

/// An empty, run-only, layer for compiled Go app binaries.
pub(crate) struct TargetLayer {}

#[derive(Error, Debug)]
#[error("Couldn't write to target layer: {0}")]
pub(crate) struct TargetLayerError(io::Error);

impl Layer for TargetLayer {
    type Buildpack = GoBuildpack;
    type Metadata = GenericMetadata;

    fn types(&self) -> LayerTypes {
        LayerTypes {
            build: true,
            launch: true,
            cache: false,
        }
    }

    // This layer creates the `GOBIN` directory, which is the target for `go install` later.
    fn create(
        &self,
        _context: &BuildContext<Self::Buildpack>,
        layer_path: &Path,
    ) -> Result<LayerResult<Self::Metadata>, GoBuildpackError> {
        let bin_dir = layer_path.join("bin");
        fs::create_dir(&bin_dir).map_err(TargetLayerError)?;
        LayerResultBuilder::new(GenericMetadata::default())
            .env(LayerEnv::new().chainable_insert(
                Scope::Build,
                libcnb::layer_env::ModificationBehavior::Override,
                "GOBIN",
                bin_dir,
            ))
            .build()
    }
}
