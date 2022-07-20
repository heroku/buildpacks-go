use crate::{cmd, GoBuildpack, GoBuildpackError};
use libcnb::build::BuildContext;
use libcnb::data::layer_content_metadata::LayerTypes;
use libcnb::layer::{ExistingLayerStrategy, Layer, LayerData, LayerResult, LayerResultBuilder};
use libcnb::layer_env::{LayerEnv, Scope};
use libcnb::{Buildpack, Env};
use libherokubuildpack::log_info;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use thiserror::Error;

/// A layer that caches the go modules cache
pub(crate) struct DepsLayer {
    pub(crate) go_env: Env,
}

const LAYER_VERSION: f32 = 1.0;
const MAX_CACHE_USAGE_COUNT: f32 = 3.0;
const CACHE_ENV: &str = "GOMODCACHE";
const CACHE_DIR: &str = "cache";

// Using float fields here due to [an issue with lifecycle's handling of integers](https://github.com/buildpacks/lifecycle/issues/884)
#[derive(Deserialize, Serialize, Clone, PartialEq)]
pub(crate) struct DepsLayerMetadata {
    cache_usage_count: f32,
    layer_version: f32,
}

#[derive(Error, Debug)]
pub(crate) enum DepsLayerError {
    #[error("Couldn't create Go modules cache layer: {0}")]
    Create(std::io::Error),
    #[error("Couldn't clean Go modules cache: {0}")]
    Clean(#[from] cmd::Error),
}

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
        _ctx: &BuildContext<Self::Buildpack>,
        layer_path: &Path,
    ) -> Result<LayerResult<Self::Metadata>, GoBuildpackError> {
        log_info("Creating new Go modules cache");
        let cache_dir = layer_path.join(CACHE_DIR);
        fs::create_dir(&cache_dir).map_err(DepsLayerError::Create)?;
        LayerResultBuilder::new(DepsLayerMetadata {
            cache_usage_count: 1.0,
            layer_version: LAYER_VERSION,
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
        &self,
        _ctx: &BuildContext<Self::Buildpack>,
        layer: &LayerData<Self::Metadata>,
    ) -> Result<LayerResult<Self::Metadata>, GoBuildpackError> {
        LayerResultBuilder::new(DepsLayerMetadata {
            cache_usage_count: layer.content_metadata.metadata.cache_usage_count + 1.0,
            layer_version: LAYER_VERSION,
        })
        .build()
    }

    fn existing_layer_strategy(
        &self,
        _ctx: &BuildContext<Self::Buildpack>,
        layer: &LayerData<Self::Metadata>,
    ) -> Result<ExistingLayerStrategy, <Self::Buildpack as Buildpack>::Error> {
        if layer.content_metadata.metadata.cache_usage_count >= MAX_CACHE_USAGE_COUNT
            || layer.content_metadata.metadata.layer_version != LAYER_VERSION
        {
            log_info("Expired Go modules cache");
            // Go restricts write permissions in cache folders, which blocks libcnb
            // from deleting the layer in a `Recreate` scenario. Go clean will
            // delete the entire cache, including the restricted access files.
            let mut go_env = self.go_env.clone();
            go_env.insert(CACHE_ENV, layer.path.join(CACHE_DIR));
            cmd::go_clean("-modcache", &go_env).map_err(DepsLayerError::Clean)?;
            return Ok(ExistingLayerStrategy::Recreate);
        }
        log_info("Reusing Go modules cache");
        Ok(ExistingLayerStrategy::Update)
    }
}
