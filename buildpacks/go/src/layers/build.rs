use crate::{GoBuildpack, GoBuildpackError};
use heroku_go_utils::vrs::GoVersion;
use heroku_inventory_utils::inv::Artifact;
use libcnb::build::BuildContext;
use libcnb::data::layer_content_metadata::LayerTypes;
use libcnb::layer::{ExistingLayerStrategy, Layer, LayerData, LayerResult, LayerResultBuilder};
use libcnb::layer_env::{LayerEnv, Scope};
use libcnb::Buildpack;
use libherokubuildpack::log::log_info;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::fs;
use std::path::Path;

/// A layer for go incremental build cache artifacts
pub(crate) struct BuildLayer {
    pub(crate) artifact: Artifact<GoVersion, Sha256>,
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
pub(crate) struct BuildLayerMetadata {
    layer_version: String,
    artifact: Artifact<GoVersion, Sha256>,
    cache_usage_count: f32,
}

#[derive(thiserror::Error, Debug)]
#[error("Couldn't write to build layer: {0}")]
pub(crate) struct BuildLayerError(std::io::Error);

const CACHE_ENV: &str = "GOCACHE";
const CACHE_DIR: &str = "cache";
const LAYER_VERSION: &str = "1";
const MAX_CACHE_USAGE_COUNT: f32 = 200.0;

impl Layer for BuildLayer {
    type Buildpack = GoBuildpack;
    type Metadata = BuildLayerMetadata;

    fn types(&self) -> LayerTypes {
        LayerTypes {
            build: true,
            launch: false,
            cache: true,
        }
    }

    fn create(
        &mut self,
        _ctx: &BuildContext<Self::Buildpack>,
        layer_path: &Path,
    ) -> Result<LayerResult<Self::Metadata>, GoBuildpackError> {
        log_info("Creating Go build cache");
        let cache_dir = layer_path.join(CACHE_DIR);
        fs::create_dir(&cache_dir).map_err(BuildLayerError)?;
        LayerResultBuilder::new(BuildLayerMetadata {
            artifact: self.artifact.clone(),
            layer_version: LAYER_VERSION.to_string(),
            cache_usage_count: 1.0,
        })
        .env(LayerEnv::new().chainable_insert(
            Scope::Build,
            libcnb::layer_env::ModificationBehavior::Override,
            CACHE_ENV,
            cache_dir,
        ))
        .build()
    }

    fn update(
        &mut self,
        _ctx: &BuildContext<Self::Buildpack>,
        layer: &LayerData<Self::Metadata>,
    ) -> Result<LayerResult<Self::Metadata>, GoBuildpackError> {
        LayerResultBuilder::new(BuildLayerMetadata {
            artifact: self.artifact.clone(),
            layer_version: LAYER_VERSION.to_string(),
            cache_usage_count: layer.content_metadata.metadata.cache_usage_count + 1.0,
        })
        .env(LayerEnv::new().chainable_insert(
            Scope::Build,
            libcnb::layer_env::ModificationBehavior::Override,
            CACHE_ENV,
            layer.path.join(CACHE_DIR),
        ))
        .build()
    }

    fn existing_layer_strategy(
        &mut self,
        _ctx: &BuildContext<Self::Buildpack>,
        layer: &LayerData<Self::Metadata>,
    ) -> Result<ExistingLayerStrategy, <Self::Buildpack as Buildpack>::Error> {
        let mdata = &layer.content_metadata.metadata;
        if mdata.cache_usage_count >= MAX_CACHE_USAGE_COUNT
            || mdata.layer_version != LAYER_VERSION
            || mdata.artifact != self.artifact
        {
            log_info("Expired Go build cache");
            return Ok(ExistingLayerStrategy::Recreate);
        }
        log_info("Reusing Go build cache");
        Ok(ExistingLayerStrategy::Update)
    }
}
