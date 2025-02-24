use crate::{GoBuildpack, GoBuildpackError};
use libcnb::build::BuildContext;
use libcnb::data::layer_name;
use libcnb::layer::UncachedLayerDefinition;
use libcnb::layer_env::{LayerEnv, Scope};
use std::fs;
use std::io;

#[derive(thiserror::Error, Debug)]
#[error("Couldn't write to target layer: {0}")]
pub(crate) struct TargetLayerError(io::Error);

impl From<TargetLayerError> for libcnb::Error<GoBuildpackError> {
    fn from(value: TargetLayerError) -> Self {
        libcnb::Error::BuildpackError(GoBuildpackError::TargetLayer(value))
    }
}

// Create the layer for compiled Go binaries
pub(crate) fn handle_target_layer(
    context: &BuildContext<GoBuildpack>,
) -> libcnb::Result<LayerEnv, GoBuildpackError> {
    let layer_ref = context.uncached_layer(
        layer_name!("go_target"),
        UncachedLayerDefinition {
            build: true,
            launch: true,
        },
    )?;
    let bin_dir = layer_ref.path().join("bin");
    fs::create_dir(&bin_dir).map_err(TargetLayerError)?;
    layer_ref.write_env(LayerEnv::new().chainable_insert(
        Scope::Build,
        libcnb::layer_env::ModificationBehavior::Override,
        "GOBIN",
        bin_dir,
    ))?;
    layer_ref.read_env()
}
