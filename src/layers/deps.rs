use crate::{GoBuildpack, GoBuildpackError};
use libcnb::build::BuildContext;
use libcnb::data::layer_content_metadata::LayerTypes;
use libcnb::layer::{ExistingLayerStrategy, Layer, LayerData, LayerResult, LayerResultBuilder};
use libcnb::Buildpack;
use libherokubuildpack::log_info;
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

/// A layer that downloads and installs the Go distribution artifacts
pub struct DepsLayer {}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct DepsLayerMetadata {
    layer_version: String,
    modtxt_sha: Option<String>,
    gosum_sha: Option<String>,
    gomod_sha: Option<String>,
}

#[derive(Error, Debug)]
pub enum DepsLayerError {
    #[error("Unknown: {0}")]
    Unknown(std::io::Error),
}

const LAYER_VERSION: &str = "1";

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
        &self,
        context: &BuildContext<Self::Buildpack>,
        layer_path: &Path,
    ) -> Result<LayerResult<Self::Metadata>, GoBuildpackError> {
        LayerResultBuilder::new(DepsLayerMetadata::current(self, context)).build()
    }

    fn existing_layer_strategy(
        &self,
        context: &BuildContext<Self::Buildpack>,
        layer_data: &LayerData<Self::Metadata>,
    ) -> Result<ExistingLayerStrategy, <Self::Buildpack as Buildpack>::Error> {
        if layer_data.content_metadata.metadata == DepsLayerMetadata::current(self, context) {
            log_info("Reusing cached Go modules");
            Ok(ExistingLayerStrategy::Keep)
        } else {
            log_info("Updating cached Go modules");
            Ok(ExistingLayerStrategy::Update)
        }
    }
}

impl DepsLayerMetadata {
    fn current(layer: &DepsLayer, context: &BuildContext<GoBuildpack>) -> Self {
        DepsLayerMetadata {
            gosum_sha: None,
            gomod_sha: None,
            modtxt_sha: None,
            layer_version: String::from(LAYER_VERSION),
        }
    }
}
