use crate::{tgz, GoBuildpack, GoBuildpackError};
use heroku_go_utils::vrs::GoVersion;
use libcnb::build::BuildContext;
use libcnb::data::layer_content_metadata::LayerTypes;
use libcnb::layer::{ExistingLayerStrategy, Layer, LayerData, LayerResult, LayerResultBuilder};
use libcnb::layer_env::{LayerEnv, ModificationBehavior, Scope};
use libcnb::Buildpack;
use libherokubuildpack::inventory::artifact::Artifact;
use libherokubuildpack::log::log_info;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::path::Path;

/// A layer that downloads and installs the Go distribution artifacts
pub(crate) struct DistLayer {
    pub(crate) artifact: Artifact<GoVersion, Sha256, Option<()>>,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq)]
pub(crate) struct DistLayerMetadata {
    layer_version: String,
    artifact: Artifact<GoVersion, Sha256, Option<()>>,
}

#[derive(thiserror::Error, Debug)]
pub(crate) enum DistLayerError {
    #[error("Couldn't extract Go distribution archive: {0}")]
    Tgz(tgz::Error),
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
        &mut self,
        _ctx: &BuildContext<Self::Buildpack>,
        layer_path: &Path,
    ) -> Result<LayerResult<Self::Metadata>, GoBuildpackError> {
        log_info(format!(
            "Installing {} ({}-{}) from {}",
            self.artifact.version, self.artifact.os, self.artifact.arch, self.artifact.url
        ));
        tgz::fetch_strip_filter_extract_verify(
            &self.artifact,
            "go",
            ["bin", "src", "pkg", "go.env", "LICENSE"].into_iter(),
            layer_path,
        )
        .map_err(DistLayerError::Tgz)?;

        LayerResultBuilder::new(DistLayerMetadata::current(self))
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
        &mut self,
        _ctx: &BuildContext<Self::Buildpack>,
        layer_data: &LayerData<Self::Metadata>,
    ) -> Result<ExistingLayerStrategy, <Self::Buildpack as Buildpack>::Error> {
        if layer_data.content_metadata.metadata == DistLayerMetadata::current(self) {
            log_info(format!(
                "Reusing {} ({}-{})",
                self.artifact.version, self.artifact.os, self.artifact.arch
            ));
            Ok(ExistingLayerStrategy::Keep)
        } else {
            Ok(ExistingLayerStrategy::Recreate)
        }
    }
}

impl DistLayerMetadata {
    fn current(layer: &DistLayer) -> Self {
        DistLayerMetadata {
            artifact: layer.artifact.clone(),
            layer_version: String::from(LAYER_VERSION),
        }
    }
}
