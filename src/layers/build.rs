use crate::{GoBuildpack, GoBuildpackError};
use libcnb::build::BuildContext;
use libcnb::data::buildpack::StackId;
use libcnb::data::layer_content_metadata::LayerTypes;
use libcnb::layer::{ExistingLayerStrategy, Layer, LayerData, LayerResult, LayerResultBuilder};
use libcnb::layer_env::LayerEnv;
use libcnb::layer_env::Scope;
use libcnb::Buildpack;
use libcnb::Env;
use libherokubuildpack::log_info;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use thiserror::Error;

/// A layer that builds go binaries and caches the incremental build cache
/// artifacts.
pub struct BuildLayer {
    pub go_target: PathBuf,
    pub go_version: String,
    pub go_env: Env,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct BuildLayerMetadata {
    layer_version: String,
    go_version: String,
    stack_id: StackId,
}

#[derive(Error, Debug)]
pub enum BuildLayerError {
    #[error("Couldn't spawn `go build` command: {0}")]
    CommandStart(std::io::Error),
    #[error("Couldn't get `go build` command result: {0}")]
    CommandResult(std::io::Error),
    #[error("`go build` exit status was {0}")]
    CommandStatus(std::process::ExitStatus),
}
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
        self.execute(context, layer_path)
    }

    fn update(
        &self,
        context: &BuildContext<Self::Buildpack>,
        layer: &LayerData<Self::Metadata>,
    ) -> Result<LayerResult<Self::Metadata>, GoBuildpackError> {
        self.execute(context, &layer.path)
    }

    fn existing_layer_strategy(
        &self,
        context: &BuildContext<Self::Buildpack>,
        layer_data: &LayerData<Self::Metadata>,
    ) -> Result<ExistingLayerStrategy, <Self::Buildpack as Buildpack>::Error> {
        if layer_data.content_metadata.metadata == BuildLayerMetadata::current(self, context) {
            log_info("Reusing Go build cache");
            Ok(ExistingLayerStrategy::Update)
        } else {
            Ok(ExistingLayerStrategy::Recreate)
        }
    }
}

impl BuildLayer {
    fn execute(
        &self,
        context: &BuildContext<GoBuildpack>,
        layer_path: &Path,
    ) -> Result<LayerResult<BuildLayerMetadata>, GoBuildpackError> {
        let mut env = self.go_env.clone();
        env.insert("GOCACHE", layer_path);
        let mut build_cmd = Command::new("go")
            .args(vec!["build", "-o", &self.go_target.to_string_lossy()])
            .envs(&env)
            .spawn()
            .map_err(BuildLayerError::CommandStart)?;

        let status = build_cmd.wait().map_err(BuildLayerError::CommandResult)?;

        status
            .success()
            .then(|| ())
            .ok_or(BuildLayerError::CommandStatus(status))?;

        LayerResultBuilder::new(BuildLayerMetadata::current(self, context)).build()
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
