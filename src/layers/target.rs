use crate::{GoBuildpack, GoBuildpackError};
use libcnb::build::BuildContext;
use libcnb::data::layer_content_metadata::LayerTypes;
use libcnb::generic::GenericMetadata;
use libcnb::layer::{Layer, LayerResult, LayerResultBuilder};
use std::path::Path;

/// An empty, run-only, layer for compiled Go app binaries.
pub struct TargetLayer {}

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

    fn create(
        &self,
        _context: &BuildContext<Self::Buildpack>,
        _layer_path: &Path,
    ) -> Result<LayerResult<Self::Metadata>, GoBuildpackError> {
        LayerResultBuilder::new(GenericMetadata::default()).build()
    }
}
