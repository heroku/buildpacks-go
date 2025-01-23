use crate::{cmd, GoBuildpack, GoBuildpackError};
use bullet_stream::state::SubBullet;
use bullet_stream::Print;
use cache_diff::CacheDiff;
use commons::layer::diff_migrate::DiffMigrateLayer;
use libcnb::build::BuildContext;
use libcnb::data::layer_content_metadata::LayerTypes;
use libcnb::data::layer_name;
use libcnb::layer::{
    EmptyLayerCause, ExistingLayerStrategy, Layer, LayerData, LayerResult, LayerResultBuilder,
    LayerState,
};
use libcnb::layer_env::{LayerEnv, Scope};
use libcnb::{Buildpack, Env};
use libherokubuildpack::log::log_info;
use magic_migrate::TryMigrate;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::Path;

const LAYER_VERSION: &str = "1";
const MAX_CACHE_USAGE_COUNT: f32 = 100.0;
const CACHE_ENV: &str = "GOMODCACHE";
const CACHE_DIR: &str = "cache";

pub(crate) fn call<W>(
    context: &BuildContext<GoBuildpack>,
    mut bullet: Print<SubBullet<W>>,
    metadata: &DepsLayerMetadata,
) -> libcnb::Result<(Print<SubBullet<W>>, LayerEnv), <GoBuildpack as Buildpack>::Error>
where
    W: Write + Send + Sync + 'static,
{
    let layer_ref = DiffMigrateLayer {
        build: true,
        launch: true,
    }
    .cached_layer(layer_name!("go_deps"), context, metadata)?;
    match &layer_ref.state {
        LayerState::Restored { cause } => {
            bullet = bullet.sub_bullet(cause);
        }
        LayerState::Empty { cause } => {
            match cause {
                EmptyLayerCause::NewlyCreated => {}
                EmptyLayerCause::InvalidMetadataAction { cause }
                | EmptyLayerCause::RestoredLayerAction { cause } => {
                    bullet = bullet.sub_bullet(cause);
                }
            }
            let timer = bullet.start_timer(format!(""));
            todo!();

            bullet = timer.done();
        }
    }

    layer_ref.write_env(todo!())?;
    Ok((bullet, layer_ref.read_env()?))
}

/// A layer that caches the go modules cache
pub(crate) struct DepsLayer {
    pub(crate) go_env: Env,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub(crate) struct DepsLayerMetadata {
    // Using float here due to [an issue with lifecycle's handling of integers](https://github.com/buildpacks/lifecycle/issues/884)
    cache_usage_count: f32,
    layer_version: String,
}

impl CacheDiff for DepsLayerMetadata {
    fn diff(&self, old: &Self) -> Vec<String> {
        let mut diff = Vec::new();
        if self.cache_usage_count >= MAX_CACHE_USAGE_COUNT {
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

magic_migrate::try_migrate_toml_chain!(
    error: MigrationError,
    chain: [DepsLayerMetadata]
);

#[derive(Debug, thiserror::Error)]
pub(crate) enum MigrationError {}

#[derive(thiserror::Error, Debug)]
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
        &mut self,
        _ctx: &BuildContext<Self::Buildpack>,
        layer_path: &Path,
    ) -> Result<LayerResult<Self::Metadata>, GoBuildpackError> {
        log_info("Creating new Go modules cache");
        let cache_dir = layer_path.join(CACHE_DIR);
        fs::create_dir(&cache_dir).map_err(DepsLayerError::Create)?;
        LayerResultBuilder::new(DepsLayerMetadata {
            cache_usage_count: 1.0,
            layer_version: LAYER_VERSION.to_string(),
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
        LayerResultBuilder::new(DepsLayerMetadata {
            cache_usage_count: layer.content_metadata.metadata.cache_usage_count + 1.0,
            layer_version: LAYER_VERSION.to_string(),
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
