use crate::{tgz, GoBuildpack, GoBuildpackError};
use heroku_go_buildpack::inv::Artifact;
use libcnb::build::BuildContext;
use libcnb::data::layer_content_metadata::LayerTypes;
use libcnb::layer::{ExistingLayerStrategy, Layer, LayerData, LayerResult, LayerResultBuilder};
use libcnb::layer_env::{LayerEnv, ModificationBehavior, Scope};
use libcnb::Buildpack;
use libherokubuildpack::log::log_info;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// A layer that downloads and installs the Go distribution artifacts
pub(crate) struct DistLayer {
    pub artifact: Artifact,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq)]
pub(crate) struct DistLayerMetadata {
    layer_version: String,
    go_version: String,
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
        &self,
        _ctx: &BuildContext<Self::Buildpack>,
        layer_path: &Path,
    ) -> Result<LayerResult<Self::Metadata>, GoBuildpackError> {
        log_info(format!("Installing Go {}", self.artifact.semantic_version));
        tgz::fetch_strip_filter_extract_verify(
            self.artifact.mirror_tarball_url(),
            "go",
            ["bin", "src", "pkg", "go.env", "LICENSE"].into_iter(),
            layer_path,
            &self.artifact.sha_checksum,
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
        &self,
        _ctx: &BuildContext<Self::Buildpack>,
        layer_data: &LayerData<Self::Metadata>,
    ) -> Result<ExistingLayerStrategy, <Self::Buildpack as Buildpack>::Error> {
        if layer_data.content_metadata.metadata == DistLayerMetadata::current(self) {
            log_info(format!("Reusing Go {}", self.artifact.semantic_version));
            Ok(ExistingLayerStrategy::Keep)
        } else {
            Ok(ExistingLayerStrategy::Recreate)
        }
    }
}

impl DistLayerMetadata {
    fn current(layer: &DistLayer) -> Self {
        DistLayerMetadata {
            go_version: layer.artifact.go_version.clone(),
            layer_version: String::from(LAYER_VERSION),
        }
    }
}
