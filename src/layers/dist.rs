use crate::{GoBuildpack, GoBuildpackError};
use heroku_go_buildpack::godist;
use heroku_go_buildpack::inv::Artifact;
use libcnb::build::BuildContext;
use libcnb::data::buildpack::StackId;
use libcnb::data::layer_content_metadata::LayerTypes;
use libcnb::layer::{ExistingLayerStrategy, Layer, LayerData, LayerResult, LayerResultBuilder};
use libcnb::layer_env::{LayerEnv, ModificationBehavior, Scope};
use libcnb::Buildpack;
use libherokubuildpack::log_info;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// A layer that downloads and installs the Go distribution artifacts
pub struct DistLayer {
    pub artifact: Artifact,
    pub tmp_dir: PathBuf,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct DistLayerMetadata {
    layer_version: String,
    go_version: String,
    stack_id: StackId,
}

#[derive(Error, Debug)]
pub enum DistLayerError {
    #[error("Couldn't install Go distribution: {0}")]
    Dist(godist::DistError),
    #[error("Couldn't create Go distribiton directory: {0}")]
    Dir(std::io::Error),
}

const LAYER_VERSION: &str = "1";

impl Layer for DistLayer {
    type Buildpack = GoBuildpack;
    type Metadata = DistLayerMetadata;

    fn types(&self) -> LayerTypes {
        LayerTypes {
            build: true,
            launch: false,
            cache: true,
        }
    }

    fn create(
        &self,
        context: &BuildContext<Self::Buildpack>,
        layer_path: &Path,
    ) -> Result<LayerResult<Self::Metadata>, GoBuildpackError> {
        log_info(format!("Installing Go {}", self.artifact.semantic_version));
        godist::fetch_strip_filter_extract_verify(
            self.artifact.mirror_tarball_url(),
            "go",
            ["bin", "src", "LICENSE"].into_iter(),
            layer_path,
            &self.artifact.sha_checksum,
        )
        .map_err(DistLayerError::Dist)?;

        LayerResultBuilder::new(DistLayerMetadata::current(self, context))
            .env(
                LayerEnv::new()
                    .chainable_insert(
                        Scope::Build,
                        ModificationBehavior::Override,
                        "GOROOT",
                        layer_path,
                    )
                    .chainable_insert(
                        Scope::Build,
                        ModificationBehavior::Override,
                        "GO111MODULE",
                        "on",
                    ),
            )
            .build()
    }

    fn existing_layer_strategy(
        &self,
        context: &BuildContext<Self::Buildpack>,
        layer_data: &LayerData<Self::Metadata>,
    ) -> Result<ExistingLayerStrategy, <Self::Buildpack as Buildpack>::Error> {
        if layer_data.content_metadata.metadata == DistLayerMetadata::current(self, context) {
            log_info(format!("Reusing Go {}", self.artifact.semantic_version));
            Ok(ExistingLayerStrategy::Keep)
        } else {
            Ok(ExistingLayerStrategy::Recreate)
        }
    }
}

impl DistLayerMetadata {
    fn current(layer: &DistLayer, context: &BuildContext<GoBuildpack>) -> Self {
        DistLayerMetadata {
            go_version: layer.artifact.go_version.clone(),
            stack_id: context.stack_id.clone(),
            layer_version: String::from(LAYER_VERSION),
        }
    }
}
