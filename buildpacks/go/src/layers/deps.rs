use crate::{GoBuildpack, GoBuildpackError};
use libcnb::build::BuildContext;
use libcnb::data::layer_name;
use libcnb::layer::{
    CachedLayerDefinition, EmptyLayerCause, InvalidMetadataAction, LayerState, RestoredLayerAction,
};
use libcnb::layer_env::{LayerEnv, Scope};
use libherokubuildpack::log::log_info;
use serde::{Deserialize, Serialize};
use std::fs;

const MAX_CACHE_USAGE_COUNT: f32 = 100.0;
const CACHE_ENV: &str = "GOMODCACHE";
const CACHE_DIR: &str = "cache";

#[derive(Deserialize, Serialize, Clone, PartialEq)]
pub(crate) struct DepsLayerMetadata {
    // Using float here due to [an issue with lifecycle's handling of integers](https://github.com/buildpacks/lifecycle/issues/884)
    cache_usage_count: f32,
}

#[derive(thiserror::Error, Debug)]
pub(crate) enum DepsLayerError {
    #[error("Couldn't create Go modules cache layer: {0}")]
    Create(std::io::Error),
}

/// Create or restore the layer for the go modules cache (non-vendored dependencies)
pub(crate) fn handle_deps_layer(
    context: &BuildContext<GoBuildpack>,
) -> libcnb::Result<LayerEnv, GoBuildpackError> {
    let layer_ref = context.cached_layer(
        layer_name!("go_deps"),
        CachedLayerDefinition {
            build: true,
            launch: false,
            invalid_metadata_action: &|_| InvalidMetadataAction::DeleteLayer,
            restored_layer_action: &|restored_metadata: &DepsLayerMetadata, _| {
                if restored_metadata.cache_usage_count >= MAX_CACHE_USAGE_COUNT {
                    return (
                        RestoredLayerAction::DeleteLayer,
                        restored_metadata.cache_usage_count,
                    );
                }
                (
                    RestoredLayerAction::KeepLayer,
                    restored_metadata.cache_usage_count,
                )
            },
        },
    )?;

    match layer_ref.state {
        LayerState::Empty {
            cause: EmptyLayerCause::NewlyCreated,
        } => (),
        LayerState::Empty {
            cause: EmptyLayerCause::RestoredLayerAction { .. },
        } => log_info("Discarding expired Go modules cache"),
        LayerState::Empty { .. } => log_info("Discarding invalid Go modules cache"),
        LayerState::Restored { .. } => log_info("Reusing Go modules cache"),
    }

    let mut cache_usage_count = 1.0;
    match layer_ref.state {
        LayerState::Restored {
            cause: previous_cache_usage_count,
        } => cache_usage_count += previous_cache_usage_count,
        LayerState::Empty { .. } => {
            log_info("Creating new Go modules cache");
            let cache_dir = layer_ref.path().join(CACHE_DIR);
            fs::create_dir(&cache_dir).map_err(DepsLayerError::Create)?;
            layer_ref.write_env(LayerEnv::new().chainable_insert(
                Scope::Build,
                libcnb::layer_env::ModificationBehavior::Override,
                CACHE_ENV,
                cache_dir,
            ))?;
        }
    }
    layer_ref.write_metadata(DepsLayerMetadata { cache_usage_count })?;
    layer_ref.read_env()
}

impl From<DepsLayerError> for libcnb::Error<GoBuildpackError> {
    fn from(value: DepsLayerError) -> Self {
        libcnb::Error::BuildpackError(GoBuildpackError::DepsLayer(value))
    }
}
