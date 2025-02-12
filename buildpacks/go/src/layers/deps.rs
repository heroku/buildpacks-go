use crate::{cmd, GoBuildpack, GoBuildpackError};
use bullet_stream::state::SubBullet;
use bullet_stream::Print;
use cache_diff::CacheDiff;
use commons::layer::diff_migrate::{DiffMigrateLayer, Meta};
use fs_err as fs;
use libcnb::build::BuildContext;
use libcnb::data::layer_name;
use libcnb::layer::{EmptyLayerCause, LayerState};
use libcnb::layer_env::{LayerEnv, ModificationBehavior, Scope};
use libcnb::Buildpack;
use magic_migrate::TryMigrate;
use serde::{Deserialize, Serialize};
use std::io::Write;

const LAYER_VERSION: &str = "1";
const MAX_CACHE_USAGE_COUNT: f32 = 100.0;
const CACHE_ENV: &str = "GOMODCACHE";
const CACHE_DIR: &str = "cache";

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
    .cached_layer(layer_name!("go_deps"), context, metadata)?;

    let cache_dir = layer_ref.path().join(CACHE_DIR);
    let layer_env = LayerEnv::new().chainable_insert(
        Scope::Build,
        ModificationBehavior::Override,
        CACHE_ENV,
        &cache_dir,
    );

    match &layer_ref.state {
        LayerState::Restored { cause } => {
            bullet = bullet.sub_bullet(cause);
            match cause {
                Meta::Message(m) => unreachable!("Should never receive an Meta::Message in LayerState::Restored when using DiffMigrateLayer. Message: {m}"),
                Meta::Data(previous) => {
                    layer_ref
                        .write_metadata(Metadata::new(previous.cache_usage_count + 1.0))?;
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
            bullet = bullet.sub_bullet("Creating go modules cache");
            fs::create_dir(&cache_dir)
                .map_err(DepsLayerError::Create)
                .map_err(GoBuildpackError::DepsLayer)?;
        }
    }
    layer_ref.write_env(layer_env)?;
    Ok((bullet, layer_ref.read_env()?))
}

impl Metadata {
    pub(crate) fn new(usage_count: f32) -> Self {
        Self {
            cache_usage_count: usage_count,
            layer_version: LAYER_VERSION.to_string(),
        }
    }
}

/// A layer that caches the go modules cache
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, TryMigrate)]
#[serde(deny_unknown_fields)]
#[try_migrate(from = None)]
pub(crate) struct MetadataV1 {
    // Using float here due to [an issue with lifecycle's handling of integers](https://github.com/buildpacks/lifecycle/issues/884)
    cache_usage_count: f32,
    layer_version: String,
}
pub(crate) type Metadata = MetadataV1;

impl CacheDiff for Metadata {
    fn diff(&self, old: &Self) -> Vec<String> {
        let mut diff = Vec::new();
        if old.cache_usage_count >= MAX_CACHE_USAGE_COUNT {
            diff.push(format!("Max cache usage reached ({MAX_CACHE_USAGE_COUNT})"));
        }

        if self.layer_version != old.layer_version {
            diff.push(format!(
                "Buildpack author triggered (v{} to v{})",
                old.layer_version, self.layer_version
            ));
        }
        diff
    }
}

#[derive(thiserror::Error, Debug)]
pub(crate) enum DepsLayerError {
    #[error("Couldn't create Go modules cache layer: {0}")]
    Create(std::io::Error),
    #[error("Couldn't clean Go modules cache: {0}")]
    Clean(#[from] cmd::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    /// See [`crate::build::tests::metadata_guard`] for info on what to do when this fails
    #[test]
    fn metadata_guard() {
        let metadata = MetadataV1 {
            cache_usage_count: 1.0,
            layer_version: LAYER_VERSION.to_string(),
        };

        let toml = r#"
cache_usage_count = 1.0
layer_version = "1"
        "#
        .trim();
        assert_eq!(
            toml,
            toml::to_string(&metadata)
                .unwrap()
                .to_string()
                .as_str()
                .trim()
        );

        assert_eq!(metadata, toml::from_str(toml).unwrap());
    }
}
