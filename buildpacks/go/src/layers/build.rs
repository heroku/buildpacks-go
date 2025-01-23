use crate::{GoBuildpack, GoBuildpackError};
use heroku_go_utils::vrs::GoVersion;
use libcnb::build::BuildContext;
use libcnb::data::layer_content_metadata::LayerTypes;
use libcnb::layer::{ExistingLayerStrategy, Layer, LayerData, LayerResult, LayerResultBuilder};
use libcnb::layer_env::{LayerEnv, Scope};
use libcnb::{Buildpack, Target};
use libherokubuildpack::log::log_info;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// A layer for go incremental build cache artifacts
pub(crate) struct BuildLayer {
    pub(crate) go_version: GoVersion,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub(crate) struct BuildLayerMetadata {
    layer_version: String,
    go_major_version: GoVersion,
    target_arch: String,
    target_distro_name: String,
    target_distro_version: String,
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
        ctx: &BuildContext<Self::Buildpack>,
        layer_path: &Path,
    ) -> Result<LayerResult<Self::Metadata>, GoBuildpackError> {
        log_info("Creating Go build cache");
        let cache_dir = layer_path.join(CACHE_DIR);
        fs::create_dir(&cache_dir).map_err(BuildLayerError)?;
        LayerResultBuilder::new(self.generate_layer_metadata(&ctx.target, 1.0))
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
        ctx: &BuildContext<Self::Buildpack>,
        layer: &LayerData<Self::Metadata>,
    ) -> Result<LayerResult<Self::Metadata>, GoBuildpackError> {
        LayerResultBuilder::new(self.generate_layer_metadata(
            &ctx.target,
            layer.content_metadata.metadata.cache_usage_count + 1.0,
        ))
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
        ctx: &BuildContext<Self::Buildpack>,
        layer: &LayerData<Self::Metadata>,
    ) -> Result<ExistingLayerStrategy, <Self::Buildpack as Buildpack>::Error> {
        let cached_metadata = &layer.content_metadata.metadata;
        if cached_metadata.cache_usage_count >= MAX_CACHE_USAGE_COUNT {
            log_info("Discarding expired Go build cache");
            return Ok(ExistingLayerStrategy::Recreate);
        }
        let new_metadata =
            &self.generate_layer_metadata(&ctx.target, cached_metadata.cache_usage_count);

        if cached_metadata != new_metadata {
            log_info("Discarding invalid Go build cache");
            return Ok(ExistingLayerStrategy::Recreate);
        }
        log_info("Reusing existing Go build cache");
        Ok(ExistingLayerStrategy::Update)
    }
}

impl BuildLayer {
    fn generate_layer_metadata(
        &self,
        target: &Target,
        cache_usage_count: f32,
    ) -> BuildLayerMetadata {
        BuildLayerMetadata {
            layer_version: LAYER_VERSION.to_string(),
            go_major_version: self.go_version.major_release_version(),
            target_arch: target.arch.to_string(),
            target_distro_name: target.distro_name.to_string(),
            target_distro_version: target.distro_version.to_string(),
            cache_usage_count,
        }
    }
}
