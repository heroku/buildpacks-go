#![warn(clippy::pedantic)]
#![warn(clippy::cargo)]
#![allow(clippy::module_name_repetitions)]

mod layers;

use heroku_go_buildpack::gomod::read_gomod_version;
use heroku_go_buildpack::inv::Inventory;
use heroku_go_buildpack::vrs::Requirement;
use layers::{
    BuildLayer, BuildLayerError, DepsLayer, DistLayer, DistLayerError, TargetLayer,
    TargetLayerError,
};
use libcnb::build::{BuildContext, BuildResult, BuildResultBuilder};
use libcnb::data::build_plan::BuildPlanBuilder;
use libcnb::data::layer_name;
use libcnb::detect::{DetectContext, DetectResult, DetectResultBuilder};
use libcnb::generic::GenericMetadata;
use libcnb::generic::GenericPlatform;
use libcnb::layer_env::Scope;
use libcnb::{buildpack_main, Buildpack, Env};
use libherokubuildpack::{log_error, log_header, log_info};
use std::path::Path;
use thiserror::Error;

#[cfg(test)]
use libcnb_test as _;

const INVENTORY: &str = include_str!("../inventory.toml");

pub struct GoBuildpack;

impl Buildpack for GoBuildpack {
    type Platform = GenericPlatform;
    type Metadata = GenericMetadata;
    type Error = GoBuildpackError;

    fn detect(&self, context: DetectContext<Self>) -> libcnb::Result<DetectResult, Self::Error> {
        let mut plan_builder = BuildPlanBuilder::new().provides("go");

        // If there are common go artifacts, this buildpack should both
        // provide and require go so that it may be used without other
        // buildpacks.
        if ["go.mod", "main.go"]
            .map(|name| context.app_dir.join(name))
            .iter()
            .any(|path| path.exists())
        {
            plan_builder = plan_builder.requires("go");
        }

        // This buildpack may provide go when required by other buildpacks,
        // so it always explicitly passes. However, if no other group
        // buildpacks require go, group detection will fail.
        DetectResultBuilder::pass()
            .build_plan(plan_builder.build())
            .build()
    }

    fn build(&self, context: BuildContext<Self>) -> libcnb::Result<BuildResult, Self::Error> {
        let inv: Inventory = toml::from_str(INVENTORY).map_err(GoBuildpackError::InventoryParse)?;
        log_header("Determining Go version");
        let requirement = read_gomod_version(context.app_dir.join("go.mod"))
            .map_err(GoBuildpackError::VersionRequirement)?
            .unwrap_or_else(Requirement::any);

        log_info(format!("Detected Go version requirement: {requirement}"));

        let artifact = inv
            .resolve(&requirement)
            .ok_or_else(|| GoBuildpackError::VersionResolution(requirement))?;

        log_info(format!(
            "Resolved Go version: {}",
            artifact.semantic_version
        ));

        log_header("Installing Go distribution");
        let dist_layer = context.handle_layer(
            layer_name!("go-dist"),
            DistLayer {
                artifact: artifact.clone(),
            },
        )?;
        let go_env = dist_layer.env.apply(Scope::Build, &Env::from_current());

        let modules = Path::exists(&context.app_dir.join("go.mod"));
        if modules {
            let vendor = Path::exists(&context.app_dir.join("vendor").join("modules.txt"));
            if vendor {
                log_header("Using vendored Go modules");
            } else {
                log_header("Installing Go modules");
                context.handle_layer(layer_name!("go-deps"), DepsLayer {})?;
            }
        } else {
            log_info("No Go modules detected");
        }

        log_header("Building Go binaries");
        let target_layer = context.handle_layer(layer_name!("go-target"), TargetLayer {})?;
        context.handle_layer(
            layer_name!("go-build"),
            BuildLayer {
                go_env,
                go_target: target_layer.path.join("bin"),
                go_version: artifact.go_version.clone(),
            },
        )?;

        log_header("Setting process types");

        BuildResultBuilder::new().build()
    }

    fn on_error(&self, error: libcnb::Error<Self::Error>) -> i32 {
        match error {
            libcnb::Error::BuildpackError(bp_err) => {
                let err_string = bp_err.to_string();
                match bp_err {
                    GoBuildpackError::BuildLayer(_) => {
                        log_error("Go build layer error", err_string);
                        20
                    }
                    GoBuildpackError::DistLayer(_) => {
                        log_error("Go distribution layer error", err_string);
                        21
                    }
                    GoBuildpackError::TargetLayer(_) => {
                        log_error("Go target layer error", err_string);
                        22
                    }
                    GoBuildpackError::VersionRequirement(_) => {
                        log_error("Go version requirement error", err_string);
                        23
                    }
                    GoBuildpackError::InventoryParse(_) => {
                        log_error("Go inventory error", err_string);
                        24
                    }
                    GoBuildpackError::VersionResolution(_) => {
                        log_error("Go version resolution error", err_string);
                        25
                    }
                }
            }
            err => {
                log_error("Internal Buildpack Error", err.to_string());
                100
            }
        }
    }
}

#[derive(Error, Debug)]
pub enum GoBuildpackError {
    #[error("Couldn't parse go version requirement: {0}")]
    VersionRequirement(anyhow::Error),
    #[error("{0}")]
    BuildLayer(#[from] BuildLayerError),
    #[error("{0}")]
    DistLayer(#[from] DistLayerError),
    #[error("{0}")]
    TargetLayer(#[from] TargetLayerError),
    #[error("Couldn't parse go artifact inventory: {0}")]
    InventoryParse(toml::de::Error),
    #[error("Couldn't resolve go version for: {0}")]
    VersionResolution(Requirement),
}

impl From<GoBuildpackError> for libcnb::Error<GoBuildpackError> {
    fn from(e: GoBuildpackError) -> Self {
        libcnb::Error::BuildpackError(e)
    }
}

buildpack_main!(GoBuildpack);
