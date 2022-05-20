use crate::{GoBuildpack, GoBuildpackError};
use libcnb::build::BuildContext;
use libcnb::data::buildpack::StackId;
use libcnb::data::layer_content_metadata::LayerTypes;
use libcnb::layer::{ExistingLayerStrategy, Layer, LayerData, LayerResult, LayerResultBuilder};
use libcnb::Buildpack;
use libherokubuildpack::log_info;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// A cache only layer used to store go build cache data
pub struct BuildLayer {}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct BuildLayerMetadata {
    layer_version: String,
    go_version: String,
    stack_id: StackId,
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
        _layer_path: &Path,
    ) -> Result<LayerResult<Self::Metadata>, GoBuildpackError> {
        LayerResultBuilder::new(BuildLayerMetadata::current(self, context)).build()
    }

    fn existing_layer_strategy(
        &self,
        context: &BuildContext<Self::Buildpack>,
        layer_data: &LayerData<Self::Metadata>,
    ) -> Result<ExistingLayerStrategy, <Self::Buildpack as Buildpack>::Error> {
        if layer_data.content_metadata.metadata == BuildLayerMetadata::current(self, context) {
            log_info("Reusing Go build cache");
            Ok(ExistingLayerStrategy::Keep)
        } else {
            Ok(ExistingLayerStrategy::Recreate)
        }
    }
}

impl BuildLayerMetadata {
    fn current(_layer: &BuildLayer, context: &BuildContext<GoBuildpack>) -> Self {
        BuildLayerMetadata {
            go_version: "go1.16".to_string(),
            layer_version: String::from(LAYER_VERSION),
            stack_id: context.stack_id.clone(),
        }
    }
}
