#![warn(clippy::pedantic)]
#![warn(clippy::cargo)]
#![allow(clippy::module_name_repetitions)]

mod layers;

use heroku_go_buildpack::gocmd::{self, GoCmdError};
use heroku_go_buildpack::gomod::read_gomod_version;
use heroku_go_buildpack::inv::Inventory;
use heroku_go_buildpack::vrs::Requirement;
use layers::{
    BuildLayer, BuildLayerError, DepsLayer, DistLayer, DistLayerError, TargetLayer,
    TargetLayerError, TmpLayer,
};
use libcnb::build::{BuildContext, BuildResult, BuildResultBuilder};
use libcnb::data::build_plan::BuildPlanBuilder;
use libcnb::data::launch::{Launch, ProcessBuilder, ProcessType, ProcessTypeError};
use libcnb::data::layer_name;
use libcnb::detect::{DetectContext, DetectResult, DetectResultBuilder};
use libcnb::generic::GenericMetadata;
use libcnb::generic::GenericPlatform;
use libcnb::layer_env::Scope;
use libcnb::{buildpack_main, Buildpack};
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
        let tmp_layer = context.handle_layer(layer_name!("go_tmp"), TmpLayer {})?;
        let dist_layer = context.handle_layer(
            layer_name!("go_dist"),
            DistLayer {
                tmp_dir: tmp_layer.path,
                artifact: artifact.clone(),
            },
        )?;
        let mut go_env = dist_layer.env.apply_to_empty(Scope::Build);

        let modules = Path::exists(&context.app_dir.join("go.mod"));
        if modules {
            let vendor = Path::exists(&context.app_dir.join("vendor").join("modules.txt"));
            if vendor {
                log_header("Using vendored Go modules");
            } else {
                log_header("Installing Go modules");
                context.handle_layer(layer_name!("go_deps"), DepsLayer {})?;
            }
        } else {
            log_info("No Go modules detected");
        }

        log_header("Building Go packages");
        let packages = gocmd::go_list(&go_env).map_err(GoBuildpackError::GoList)?;
        log_info(format!("Detected go packages: {packages:?}"));

        let target_layer = context.handle_layer(layer_name!("go_target"), TargetLayer {})?;
        let build_layer = context.handle_layer(
            layer_name!("go_build"),
            BuildLayer {
                go_version: artifact.go_version.clone(),
            },
        )?;
        go_env = build_layer.env.apply(Scope::Build, &go_env);

        gocmd::go_build(
            &packages,
            &target_layer.path.join("bin").to_string_lossy(),
            &go_env,
        )
        .map_err(GoBuildpackError::GoBuild)?;

        log_header("Setting process types");
        let procs = packages
            .iter()
            .filter_map(|pkg| pkg.rsplit_once("/").and_then(|(_base, name)| Some(name)))
            .map(|pkg| pkg.parse())
            .collect::<Result<Vec<ProcessType>, _>>()
            .map_err(GoBuildpackError::ProcessType)?;

        let mut launch_procs = Launch::new();
        for (i, proc) in procs.iter().enumerate() {
            launch_procs.processes.push(
                ProcessBuilder::new(proc.clone(), proc.to_string())
                    .default(i == 0)
                    .build(),
            );
        }

        BuildResultBuilder::new().launch(launch_procs).build()
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
                    GoBuildpackError::GoBuild(_) => {
                        log_error("Go build error", err_string);
                        26
                    }
                    GoBuildpackError::GoList(_) => {
                        log_error("Go list error", err_string);
                        26
                    }
                    GoBuildpackError::ProcessType(_) => {
                        log_error("Go buildpack process error", err_string);
                        27
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
    #[error("Couldn't run `go build`: {0}")]
    GoBuild(GoCmdError),
    #[error("Couldn't run `go list`: {0}")]
    GoList(GoCmdError),
    #[error("{0}")]
    DistLayer(#[from] DistLayerError),
    #[error("{0}")]
    TargetLayer(#[from] TargetLayerError),
    #[error("Couldn't parse go artifact inventory: {0}")]
    InventoryParse(toml::de::Error),
    #[error("Couldn't resolve go version for: {0}")]
    VersionResolution(Requirement),
    #[error("Couldn't resolve process type: {0}")]
    ProcessType(ProcessTypeError),
}

impl From<GoBuildpackError> for libcnb::Error<GoBuildpackError> {
    fn from(e: GoBuildpackError) -> Self {
        libcnb::Error::BuildpackError(e)
    }
}

buildpack_main!(GoBuildpack);
