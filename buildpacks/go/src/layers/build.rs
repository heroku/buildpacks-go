use crate::{GoBuildpack, GoBuildpackError};
use bullet_stream::state::SubBullet;
use bullet_stream::Print;
use cache_diff::CacheDiff;
use commons::layer::diff_migrate::{DiffMigrateLayer, Meta};
use fs_err as fs;
use heroku_go_utils::vrs::GoVersion;
use libcnb::build::BuildContext;
use libcnb::data::layer_name;
use libcnb::layer::{EmptyLayerCause, LayerState};
use libcnb::layer_env::{LayerEnv, Scope};
use libcnb::{Buildpack, Target};
use magic_migrate::TryMigrate;
use serde::{Deserialize, Serialize};
use std::io::Write;

const CACHE_ENV: &str = "GOCACHE";
const CACHE_DIR: &str = "cache";
const LAYER_VERSION: &str = "1";
const MAX_CACHE_USAGE_COUNT: f32 = 200.0;

pub(crate) fn call<W>(
    context: &BuildContext<GoBuildpack>,
    mut bullet: Print<SubBullet<W>>,
    metadata: &Metadata,
) -> libcnb::Result<(Print<SubBullet<W>>, LayerEnv), <GoBuildpack as Buildpack>::Error>
where
    W: Write + Send + Sync + 'static,
{
    let layer_ref = DiffMigrateLayer {
        build: true,
        launch: false,
    }
    .cached_layer(layer_name!("go_build"), context, metadata)?;

    layer_ref.write_env(LayerEnv::new().chainable_insert(
        Scope::Build,
        libcnb::layer_env::ModificationBehavior::Override,
        CACHE_ENV,
        layer_ref.path().join(CACHE_DIR),
    ))?;
    match &layer_ref.state {
        LayerState::Restored { cause } => {
            bullet = bullet.sub_bullet(cause);
            match cause {
                Meta::Message(m) => unreachable!("Should never receive an Meta::Message in LayerState::Restored when using DiffMigrateLayer. Message: {m}"),
                Meta::Data(previous) => {
                    let mut new_metadata = metadata.clone();
                    new_metadata.cache_usage_count = previous.cache_usage_count + 1.0;
                    layer_ref
                        .write_metadata(new_metadata)?;
                }
            }
        }
        LayerState::Empty { cause } => {
            match cause {
                EmptyLayerCause::NewlyCreated => {}
                EmptyLayerCause::InvalidMetadataAction { cause }
                | EmptyLayerCause::RestoredLayerAction { cause } => {
                    bullet = bullet.sub_bullet(cause);
                }
            }
            bullet = bullet.sub_bullet("Creating cache dir");
            fs::create_dir(layer_ref.path().join(CACHE_DIR))
                .map_err(BuildLayerError)
                .map_err(GoBuildpackError::BuildLayer)?;
        }
    }

    Ok((bullet, layer_ref.read_env()?))
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, CacheDiff)]
#[cache_diff(custom = custom_cache_diff)]
pub(crate) struct MetadataV1 {
    #[cache_diff(rename = "Buildpack author triggered")]
    layer_version: String,
    go_major_version: GoVersion,
    #[cache_diff(rename = "CPU architecture")]
    target_arch: String,
    #[cache_diff(ignore = "custom")]
    target_distro_name: String,
    #[cache_diff(ignore = "custom")]
    target_distro_version: String,
    #[cache_diff(ignore = "custom")]
    cache_usage_count: f32,
}
pub(crate) type Metadata = MetadataV1;

fn custom_cache_diff(old: &Metadata, now: &Metadata) -> Vec<String> {
    let mut diff = Vec::new();
    let Metadata {
        layer_version: _,
        go_major_version: _,
        target_arch: _,
        target_distro_name,
        target_distro_version,
        cache_usage_count,
    } = old;

    if cache_usage_count >= &MAX_CACHE_USAGE_COUNT {
        diff.push(format!("Max cache usage reached ({MAX_CACHE_USAGE_COUNT})"));
    }

    if target_distro_name != &now.target_distro_name
        || target_distro_version != &now.target_distro_version
    {
        diff.push(format!(
            "OS ({}-{} to {}-{})",
            target_distro_name,
            target_distro_version,
            now.target_distro_name,
            now.target_distro_version
        ));
    }

    diff
}

magic_migrate::try_migrate_toml_chain!(
    error: MigrationError,
    chain: [Metadata]
);

#[derive(Debug, thiserror::Error)]
pub(crate) enum MigrationError {}

#[derive(thiserror::Error, Debug)]
#[error("Couldn't write to build layer: {0}")]
pub(crate) struct BuildLayerError(std::io::Error);

impl Metadata {
    pub(crate) fn new(go_version: &GoVersion, target: &Target) -> Metadata {
        Metadata {
            layer_version: LAYER_VERSION.to_string(),
            go_major_version: go_version.major_release_version(),
            target_arch: target.arch.to_string(),
            target_distro_name: target.distro_name.to_string(),
            target_distro_version: target.distro_version.to_string(),
            cache_usage_count: 1.0,
        }
    }
}
