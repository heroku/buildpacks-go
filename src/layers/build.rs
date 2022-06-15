use crate::{GoBuildpack, GoBuildpackError};
use libcnb::build::BuildContext;
use libcnb::data::buildpack::StackId;
use libcnb::data::layer_content_metadata::LayerTypes;
use libcnb::layer::{ExistingLayerStrategy, Layer, LayerData, LayerResult, LayerResultBuilder};
use libcnb::layer_env::{LayerEnv, Scope};
use libcnb::Buildpack;
use libherokubuildpack::log_info;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use thiserror::Error;

/// A layer for go incremental build cache artifacts
pub struct BuildLayer {
    pub go_version: String,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct BuildLayerMetadata {
    layer_version: String,
    go_version: String,
    stack_id: StackId,
}

#[derive(Error, Debug)]
#[error("Couldn't write to build layer: {0}")]
pub struct BuildLayerError(std::io::Error);

const LAYER_VERSION: &str = "1";

impl Layer for BuildLayer {
    type Buildpack = GoBuildpack;
    type Metadata = BuildLayerMetadata;

    fn types(&self) -> LayerTypes {
        LayerTypes {
            build: false,
            launch: false,
            cache: true,
        }
    }

    fn create(
        &self,
        context: &BuildContext<Self::Buildpack>,
        layer_path: &Path,
    ) -> Result<LayerResult<Self::Metadata>, GoBuildpackError> {
        log_info("Creating Go build cache");
        let cache_dir = layer_path.join("cache");
        fs::create_dir(&cache_dir).map_err(BuildLayerError)?;
        LayerResultBuilder::new(BuildLayerMetadata::current(self, context))
            .env(LayerEnv::new().chainable_insert(
                Scope::Build,
                libcnb::layer_env::ModificationBehavior::Override,
                "GOCACHE",
                cache_dir,
            ))
            .build()
    }

    fn existing_layer_strategy(
        &self,
        context: &BuildContext<Self::Buildpack>,
        layer_data: &LayerData<Self::Metadata>,
    ) -> Result<ExistingLayerStrategy, <Self::Buildpack as Buildpack>::Error> {
        if layer_data.content_metadata.metadata == BuildLayerMetadata::current(self, context) {
            log_info("Reusing Go build cache");
            Ok(ExistingLayerStrategy::Keep)
        } else {
            Ok(ExistingLayerStrategy::Recreate)
        }
    }
}

impl BuildLayerMetadata {
    fn current(layer: &BuildLayer, context: &BuildContext<GoBuildpack>) -> Self {
        BuildLayerMetadata {
            go_version: layer.go_version.clone(),
            layer_version: String::from(LAYER_VERSION),
            stack_id: context.stack_id.clone(),
        }
    }
}
