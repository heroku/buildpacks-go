use crate::{tgz, GoBuildpack, GoBuildpackError};
use bullet_stream::global::print;
use heroku_go_utils::vrs::GoVersion;
use libcnb::build::BuildContext;
use libcnb::data::layer_name;
use libcnb::layer::{
    CachedLayerDefinition, InvalidMetadataAction, LayerRef, LayerState, RestoredLayerAction,
};
use libcnb::layer_env::{LayerEnv, ModificationBehavior, Scope};
use libherokubuildpack::inventory::artifact::Artifact;
use serde::{Deserialize, Serialize};
use sha2::Sha256;

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq)]
pub(crate) struct DistLayerMetadata {
    artifact: Artifact<GoVersion, Sha256, Option<()>>,
}

#[derive(thiserror::Error, Debug)]
pub(crate) enum DistLayerError {
    #[error("Couldn't extract Go distribution archive: {0}")]
    Tgz(tgz::Error),
}

/// Downloads and installs the Go distribution / toolchain.
pub(crate) fn handle_dist_layer(
    context: &BuildContext<GoBuildpack>,
    artifact: &Artifact<GoVersion, Sha256, Option<()>>,
) -> libcnb::Result<LayerRef<GoBuildpack, (), ()>, GoBuildpackError> {
    let layer_ref = context.cached_layer(
        layer_name!("go_dist"),
        CachedLayerDefinition {
            build: true,
            launch: false,
            invalid_metadata_action: &|_| InvalidMetadataAction::DeleteLayer,
            restored_layer_action: &|restored_metadata: &DistLayerMetadata, _| {
                if artifact == &restored_metadata.artifact {
                    return RestoredLayerAction::KeepLayer;
                }
                RestoredLayerAction::DeleteLayer
            },
        },
    )?;

    match layer_ref.state {
        LayerState::Restored { .. } => {
            print::sub_bullet(format!(
                "Reusing {} ({}-{})",
                artifact.version, artifact.os, artifact.arch
            ));
        }
        LayerState::Empty { .. } => {
            print::sub_bullet(format!(
                "Installing {} ({}-{}) from {}",
                artifact.version, artifact.os, artifact.arch, artifact.url
            ));
            tgz::fetch_strip_filter_extract_verify(
                artifact,
                "go",
                ["bin", "src", "pkg", "go.env", "LICENSE"].into_iter(),
                layer_ref.path(),
            )
            .map_err(DistLayerError::Tgz)?;

            layer_ref.write_metadata(DistLayerMetadata {
                artifact: artifact.clone(),
            })?;
            layer_ref.write_env(
                LayerEnv::new()
                    .chainable_insert(
                        Scope::Build,
                        ModificationBehavior::Override,
                        "GOROOT",
                        layer_ref.path(),
                    )
                    .chainable_insert(
                        Scope::Build,
                        ModificationBehavior::Override,
                        "GO111MODULE",
                        "on",
                    ),
            )?;
        }
    }
    Ok(layer_ref)
}

impl From<DistLayerError> for libcnb::Error<GoBuildpackError> {
    fn from(value: DistLayerError) -> Self {
        libcnb::Error::BuildpackError(GoBuildpackError::DistLayer(value))
    }
}
