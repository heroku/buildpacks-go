use crate::{GoBuildpack, GoBuildpackError};
use heroku_go_buildpack::inv::Artifact;
use libcnb::build::BuildContext;
use libcnb::data::buildpack::StackId;
use libcnb::data::layer_content_metadata::LayerTypes;
use libcnb::layer::{ExistingLayerStrategy, Layer, LayerData, LayerResult, LayerResultBuilder};
use libcnb::layer_env::{LayerEnv, Scope};
use libcnb::Buildpack;
use libherokubuildpack::{decompress_tarball, download_file, log_info, move_directory_contents};
use serde::{Deserialize, Serialize};
use std::fs::File;
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
    #[error("Couldn't read Go distribution tarball: {0}")]
    OpenTar(std::io::Error),
    #[error("Couldn't create Go distribiton directory: {0}")]
    Dir(std::io::Error),
    #[error("Couldn't download Go distribution: {0}")]
    Download(libherokubuildpack::DownloadError),
    #[error("Couldn't extract Go distribution: {0}")]
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
            launch: false,
            cache: true,
        }
    }

    fn create(
        &self,
        context: &BuildContext<Self::Buildpack>,
        layer_path: &Path,
    ) -> Result<LayerResult<Self::Metadata>, GoBuildpackError> {
        let tmp_tgz_path = self.tmp_dir.join("go.tar.gz");

        log_info(format!("Downloading Go {}", self.artifact.semantic_version));
        download_file(self.artifact.mirror_tarball_url(), &tmp_tgz_path)
            .map_err(DistLayerError::Download)?;

        log_info(format!("Extracting Go {}", self.artifact.semantic_version));
        let mut tmp_tgz = File::open(tmp_tgz_path).map_err(DistLayerError::OpenTar)?;
        decompress_tarball(&mut tmp_tgz, &self.tmp_dir).map_err(DistLayerError::Untar)?;

        log_info(format!("Installing Go {}", self.artifact.semantic_version));
        move_directory_contents(self.tmp_dir.join("go"), layer_path)
            .map_err(DistLayerError::Installation)?;

        LayerResultBuilder::new(DistLayerMetadata::current(self, context))
            .env(LayerEnv::new().chainable_insert(
                Scope::Build,
                libcnb::layer_env::ModificationBehavior::Override,
                "GOROOT",
                layer_path,
            ))
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
