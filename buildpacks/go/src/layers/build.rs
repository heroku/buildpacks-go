use crate::{GoBuildpack, GoBuildpackError};
use bullet_stream::global::print;
use heroku_go_utils::vrs::GoVersion;
use libcnb::Target;
use libcnb::build::BuildContext;
use libcnb::data::layer_name;
use libcnb::layer::{
    CachedLayerDefinition, EmptyLayerCause, InvalidMetadataAction, LayerState, RestoredLayerAction,
};
use libcnb::layer_env::{LayerEnv, Scope};
use serde::{Deserialize, Serialize};
use std::fs;

const CACHE_ENV: &str = "GOCACHE";
const CACHE_DIR: &str = "cache";
const MAX_CACHE_USAGE_COUNT: f32 = 200.0;

#[derive(Deserialize, Serialize, Clone, PartialEq)]
pub(crate) struct BuildLayerMetadata {
    go_major_version: GoVersion,
    target_arch: String,
    target_distro_name: String,
    target_distro_version: String,
    cache_usage_count: f32,
}

impl BuildLayerMetadata {
    fn new(version: &GoVersion, target: &Target) -> Self {
        Self {
            go_major_version: version.major_release_version(),
            target_arch: target.arch.to_string(),
            target_distro_name: target.distro_name.to_string(),
            target_distro_version: target.distro_version.to_string(),
            cache_usage_count: 1.0,
        }
    }
}

#[derive(thiserror::Error, Debug)]
#[error("Couldn't write to build layer: {0}")]
pub(crate) struct BuildLayerError(std::io::Error);

impl From<BuildLayerError> for libcnb::Error<GoBuildpackError> {
    fn from(value: BuildLayerError) -> Self {
        libcnb::Error::BuildpackError(GoBuildpackError::BuildLayer(value))
    }
}

enum BuildLayerCacheState {
    Expired,
    Invalid,
    Valid,
}

/// Create or restore the layer for cached incremental build artifacts
pub(crate) fn handle_build_layer(
    context: &BuildContext<GoBuildpack>,
    go_version: &GoVersion,
) -> libcnb::Result<LayerEnv, GoBuildpackError> {
    let mut metadata = BuildLayerMetadata::new(go_version, &context.target);
    let layer_ref = context.cached_layer(
        layer_name!("go_build"),
        CachedLayerDefinition {
            build: true,
            launch: false,
            invalid_metadata_action: &|_| {
                (
                    InvalidMetadataAction::DeleteLayer,
                    BuildLayerCacheState::Invalid,
                )
            },
            restored_layer_action: &|restored_metadata: &BuildLayerMetadata, _| {
                if restored_metadata.cache_usage_count >= MAX_CACHE_USAGE_COUNT {
                    return (
                        RestoredLayerAction::DeleteLayer,
                        (
                            BuildLayerCacheState::Expired,
                            restored_metadata.cache_usage_count,
                        ),
                    );
                }
                if restored_metadata.go_major_version != metadata.go_major_version
                    || restored_metadata.target_arch != metadata.target_arch
                    || restored_metadata.target_distro_name != metadata.target_distro_name
                    || restored_metadata.target_distro_version != metadata.target_distro_version
                {
                    return (
                        RestoredLayerAction::DeleteLayer,
                        (
                            BuildLayerCacheState::Invalid,
                            restored_metadata.cache_usage_count,
                        ),
                    );
                }
                (
                    RestoredLayerAction::KeepLayer,
                    (
                        BuildLayerCacheState::Valid,
                        restored_metadata.cache_usage_count,
                    ),
                )
            },
        },
    )?;

    match layer_ref.state {
        LayerState::Empty {
            cause: EmptyLayerCause::NewlyCreated,
        } => (),
        LayerState::Empty {
            cause:
                EmptyLayerCause::RestoredLayerAction {
                    cause: (BuildLayerCacheState::Expired, _),
                },
        } => {
            print::sub_bullet("Discarding expired Go build cache");
        }
        LayerState::Empty { .. } => {
            print::sub_bullet("Discarding invalid Go build cache");
        }
        LayerState::Restored { .. } => {
            print::sub_bullet("Reusing existing Go build cache");
        }
    }

    match layer_ref.state {
        LayerState::Restored {
            cause: (_, cache_usage_count),
        } => {
            metadata.cache_usage_count += cache_usage_count;
        }
        LayerState::Empty { .. } => {
            print::sub_bullet("Creating Go build cache");
            let cache_dir = layer_ref.path().join(CACHE_DIR);
            fs::create_dir(&cache_dir).map_err(BuildLayerError)?;
            layer_ref.write_env(LayerEnv::new().chainable_insert(
                Scope::Build,
                libcnb::layer_env::ModificationBehavior::Override,
                CACHE_ENV,
                cache_dir,
            ))?;
        }
    }
    layer_ref.write_metadata(metadata)?;
    layer_ref.read_env()
}
