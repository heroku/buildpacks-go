use crate::{GoBuildpack, GoBuildpackError};
use libcnb::build::BuildContext;
use libcnb::data::buildpack::StackId;
use libcnb::data::layer_content_metadata::LayerTypes;
use libcnb::layer::{ExistingLayerStrategy, Layer, LayerData, LayerResult, LayerResultBuilder};
use libcnb::Buildpack;
use libherokubuildpack::{decompress_tarball, download_file, log_info, move_directory_contents};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tempfile::NamedTempFile;
use thiserror::Error;

/// A layer that downloads and installs the Go distribution artifacts
pub struct DistLayer {
    pub go_version: String,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct DistLayerMetadata {
    layer_version: String,
    go_version: String,
    stack_id: StackId,
}

#[derive(Error, Debug)]
pub enum DistLayerError {
    #[error("Couldn't create tempfile for Go distribution: {0}")]
    TempFile(std::io::Error),
    #[error("Couldn't download Go distribution: {0}")]
    Download(libherokubuildpack::DownloadError),
    #[error("Couldn't decompress Go distribution: {0}")]
    Untar(std::io::Error),
    #[error("Couldn't move Go distribution artifacts to the correct location: {0}")]
    Installation(std::io::Error),
}

const LAYER_VERSION: &str = "1";

impl Layer for DistLayer {
    type Buildpack = GoBuildpack;
    type Metadata = DistLayerMetadata;

    fn types(&self) -> LayerTypes {
        LayerTypes {
            build: true,
            launch: true,
            cache: true,
        }
    }

    fn create(
        &self,
        context: &BuildContext<Self::Buildpack>,
        layer_path: &Path,
    ) -> Result<LayerResult<Self::Metadata>, GoBuildpackError> {
        let go_tgz = NamedTempFile::new().map_err(DistLayerError::TempFile)?;

        log_info(format!("Downloading Go {}", self.go_version));
        download_file("some_url", go_tgz.path()).map_err(DistLayerError::Download)?;

        log_info(format!("Extracting Go {}", self.go_version));
        decompress_tarball(&mut node_tgz.into_file(), &layer_path)
            .map_err(DistLayerError::Untar)?;

        log_info(format!("Installing Go {}", self.go_version));
        let dist_path = Path::new(layer_path).join(self.go_version);
        move_directory_contents(dist_path, layer_path).map_err(DistLayerError::Installation)?;

        LayerResultBuilder::new(DistLayerMetadata::current(self, context)).build()
    }

    fn existing_layer_strategy(
        &self,
        context: &BuildContext<Self::Buildpack>,
        layer_data: &LayerData<Self::Metadata>,
    ) -> Result<ExistingLayerStrategy, <Self::Buildpack as Buildpack>::Error> {
        if layer_data.content_metadata.metadata == DistLayerMetadata::current(self, context) {
            log_info(format!("Reusing Go {}", self.go_version));
            Ok(ExistingLayerStrategy::Keep)
        } else {
            Ok(ExistingLayerStrategy::Recreate)
        }
    }
}

impl DistLayerMetadata {
    fn current(layer: &DistLayer, context: &BuildContext<GoBuildpack>) -> Self {
        DistLayerMetadata {
            go_version: layer.go_version,
            stack_id: context.stack_id.clone(),
            layer_version: String::from(LAYER_VERSION),
        }
    }
}
