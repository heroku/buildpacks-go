use crate::{tgz, GoBuildpack, GoBuildpackError};
use bullet_stream::state::SubBullet;
use bullet_stream::{style, Print};
use cache_diff::CacheDiff;
use commons::layer::diff_migrate::DiffMigrateLayer;
use heroku_go_utils::vrs::GoVersion;
use libcnb::build::BuildContext;
use libcnb::data::layer_name;
use libcnb::layer::{EmptyLayerCause, LayerState};
use libcnb::layer_env::{LayerEnv, ModificationBehavior, Scope};
use libcnb::Buildpack;
use libherokubuildpack::inventory::artifact::Artifact;
use magic_migrate::TryMigrate;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::io::Write;

pub(crate) fn call<W>(
    context: &BuildContext<GoBuildpack>,
    mut bullet: Print<SubBullet<W>>,
    metadata: &DistLayerMetadata,
) -> libcnb::Result<(Print<SubBullet<W>>, LayerEnv), <GoBuildpack as Buildpack>::Error>
where
    W: Write + Send + Sync + 'static,
{
    let layer_ref = DiffMigrateLayer {
        build: true,
        launch: false,
    }
    .cached_layer(layer_name!("go_dist"), context, metadata)?;
    match &layer_ref.state {
        LayerState::Restored { cause } => {
            bullet = bullet.sub_bullet(cause);
        }
        LayerState::Empty { cause } => {
            match cause {
                EmptyLayerCause::NewlyCreated => {}
                EmptyLayerCause::InvalidMetadataAction { cause }
                | EmptyLayerCause::RestoredLayerAction { cause } => {
                    bullet = bullet.sub_bullet(cause);
                }
            }
            let timer = bullet.start_timer(format!(
                "Installing {} ({}-{}) from {}",
                metadata.artifact.version,
                metadata.artifact.os,
                metadata.artifact.arch,
                metadata.artifact.url
            ));
            tgz::fetch_strip_filter_extract_verify(
                &metadata.artifact,
                "go",
                ["bin", "src", "pkg", "go.env", "LICENSE"].into_iter(),
                layer_ref.path(),
            )
            .map_err(DistLayerError::Tgz)
            .map_err(GoBuildpackError::DistLayer)?;

            bullet = timer.done();
        }
    }

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
    Ok((bullet, layer_ref.read_env()?))
}

/// A layer that downloads and installs the Go distribution artifacts
pub(crate) struct DistLayer {
    pub(crate) artifact: Artifact<GoVersion, Sha256, Option<()>>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub(crate) struct DistLayerMetadata {
    layer_version: String,
    artifact: Artifact<GoVersion, Sha256, Option<()>>,
}

impl CacheDiff for DistLayerMetadata {
    fn diff(&self, old: &Self) -> Vec<String> {
        let mut diff = Vec::new();
        let DistLayerMetadata {
            layer_version,
            artifact:
                Artifact {
                    version,
                    os,
                    arch,
                    url: _,
                    checksum,
                    metadata: _,
                },
        } = &self;

        if layer_version != &old.layer_version {
            diff.push(format!(
                "Layer version ({} to {})",
                style::value(&old.layer_version),
                style::value(&self.layer_version)
            ));
        }

        if version != &old.artifact.version {
            diff.push(format!(
                "Go version ({} to {})",
                style::value(old.artifact.version.to_string()),
                style::value(version.to_string())
            ));
        } else if checksum != &old.artifact.checksum {
            diff.push(format!(
                "Go binary checksum ({} to {})",
                style::value(hex::encode(&old.artifact.checksum.value)),
                style::value(hex::encode(&checksum.value))
            ));
        }

        if os != &old.artifact.os || arch != &old.artifact.arch {
            diff.push(format!(
                "OS ({}-{} to {}-{})",
                old.artifact.os, old.artifact.arch, os, arch
            ));
        }

        diff
    }
}

magic_migrate::try_migrate_toml_chain!(
    error: MigrationError,
    chain: [DistLayerMetadata]
);

#[derive(Debug, thiserror::Error)]
pub(crate) enum MigrationError {}

#[derive(thiserror::Error, Debug)]
pub(crate) enum DistLayerError {
    #[error("Couldn't extract Go distribution archive: {0}")]
    Tgz(tgz::Error),
}

const LAYER_VERSION: &str = "1";

impl DistLayerMetadata {
    pub(crate) fn new(artifact: &Artifact<GoVersion, Sha256, Option<()>>) -> Self {
        DistLayerMetadata {
            artifact: artifact.clone(),
            layer_version: String::from(LAYER_VERSION),
        }
    }
}

#[cfg(test)]
mod tests {
    use libherokubuildpack::inventory::{
        artifact::{Arch, Os},
        Inventory,
    };

    use super::*;

    fn linux_amd_artifact(version: &str) -> Artifact<GoVersion, Sha256, Option<()>> {
        let inv: Inventory<GoVersion, Sha256, Option<()>> =
            toml::from_str(include_str!("../../inventory.toml")).unwrap();

        inv.resolve(
            Os::Linux,
            Arch::Amd64,
            &semver::VersionReq::parse(version).unwrap(),
        )
        .unwrap()
        .to_owned()
    }

    #[test]
    fn test_cache_diff_go_versions() {
        let actual = DistLayerMetadata {
            layer_version: "1".to_string(),
            artifact: linux_amd_artifact("=1.22.7"),
        }
        .diff(&DistLayerMetadata {
            layer_version: "1".to_string(),
            artifact: linux_amd_artifact("= 1.23.4"),
        })
        .iter()
        .map(bullet_stream::strip_ansi)
        .collect::<Vec<_>>();

        let expected = vec!["Go version (`go1.23.4` to `go1.22.7`)".to_string()];
        assert_eq!(expected, actual);
    }
}
