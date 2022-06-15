use crate::{GoBuildpack, GoBuildpackError};
use libcnb::build::BuildContext;
use libcnb::data::layer_content_metadata::LayerTypes;
use libcnb::layer::{ExistingLayerStrategy, Layer, LayerData, LayerResult, LayerResultBuilder};
use libcnb::layer_env::{LayerEnv, Scope};
use libcnb::Buildpack;
use libherokubuildpack::log_info;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use thiserror::Error;

/// A layer that caches the go modules cache
pub struct DepsLayer {}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct DepsLayerMetadata {
    layer_version: String,
}

#[derive(Error, Debug)]
#[error("Couldn't write to build layer: {0}")]
pub struct DepsLayerError(std::io::Error);

const LAYER_VERSION: &str = "1";

impl Layer for DepsLayer {
    type Buildpack = GoBuildpack;
    type Metadata = DepsLayerMetadata;

    fn types(&self) -> LayerTypes {
        LayerTypes {
            build: true,
            launch: false,
            cache: true,
        }
    }

    fn create(
        &self,
        context: &BuildContext<Self::Buildpack>,
        layer_path: &Path,
    ) -> Result<LayerResult<Self::Metadata>, GoBuildpackError> {
        log_info("Creating Go modules cache");
        let cache_dir = layer_path.join("cache");
        fs::create_dir(&cache_dir).map_err(DepsLayerError)?;
        LayerResultBuilder::new(DepsLayerMetadata::current(self, context))
            .env(LayerEnv::new().chainable_insert(
                Scope::Build,
                libcnb::layer_env::ModificationBehavior::Override,
                "GOMODCACHE",
                cache_dir,
            ))
            .build()
    }

    fn existing_layer_strategy(
        &self,
        context: &BuildContext<Self::Buildpack>,
        layer_data: &LayerData<Self::Metadata>,
    ) -> Result<ExistingLayerStrategy, <Self::Buildpack as Buildpack>::Error> {
        if layer_data.content_metadata.metadata == DepsLayerMetadata::current(self, context) {
            log_info("Reusing Go modules cache");
            Ok(ExistingLayerStrategy::Keep)
        } else {
            Ok(ExistingLayerStrategy::Recreate)
        }
    }
}

impl DepsLayerMetadata {
    fn current(_layer: &DepsLayer, _context: &BuildContext<GoBuildpack>) -> Self {
        DepsLayerMetadata {
            layer_version: String::from(LAYER_VERSION),
        }
    }
}
