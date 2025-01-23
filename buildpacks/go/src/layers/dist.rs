use crate::{tgz, GoBuildpack, GoBuildpackError};
use bullet_stream::style;
use cache_diff::CacheDiff;
use heroku_go_utils::vrs::GoVersion;
use libcnb::build::BuildContext;
use libcnb::data::layer_content_metadata::LayerTypes;
use libcnb::layer::{ExistingLayerStrategy, Layer, LayerData, LayerResult, LayerResultBuilder};
use libcnb::layer_env::{LayerEnv, ModificationBehavior, Scope};
use libcnb::Buildpack;
use libherokubuildpack::inventory::artifact::Artifact;
use libherokubuildpack::log::log_info;
use magic_migrate::TryMigrate;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::path::Path;

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
