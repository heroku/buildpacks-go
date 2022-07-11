#![warn(clippy::pedantic)]
#![warn(clippy::cargo)]
#![allow(clippy::module_name_repetitions)]

mod layers;

use heroku_go_buildpack::cfg::read_gomod;
use heroku_go_buildpack::cmd::{self, CmdError};
use heroku_go_buildpack::inv::Inventory;
use heroku_go_buildpack::proc;
use heroku_go_buildpack::vrs::Requirement;
use layers::build::{BuildLayer, BuildLayerError};
use layers::deps::{DepsLayer, DepsLayerError};
use layers::dist::{DistLayer, DistLayerError};
use layers::target::{TargetLayer, TargetLayerError};
use libcnb::build::{BuildContext, BuildResult, BuildResultBuilder};
use libcnb::data::build_plan::BuildPlanBuilder;
use libcnb::data::launch::Launch;
use libcnb::data::layer_name;
use libcnb::detect::{DetectContext, DetectResult, DetectResultBuilder};
use libcnb::generic::GenericMetadata;
use libcnb::generic::GenericPlatform;
use libcnb::layer_env::Scope;
use libcnb::{buildpack_main, Buildpack, Env};
use libherokubuildpack::{log_error, log_header, log_info};
use std::env;
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

        // If a go.mod exists, this buildpack should both provide and require
        // go so that it may be used without other buildpacks.
        if context.app_dir.join("go.mod").exists() {
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
        log_header("Reading build configuration");

        let mut go_env = Env::new();
        env::vars()
            .filter(|(k, _v)| k == "PATH")
            .for_each(|(k, v)| {
                go_env.insert(k, v);
            });

        let inv: Inventory = toml::from_str(INVENTORY).map_err(GoBuildpackError::InventoryParse)?;

        let cfg = read_gomod(context.app_dir.join("go.mod")).map_err(GoBuildpackError::GoMod)?;
        let requirement = cfg.version.unwrap_or_else(Requirement::any);
        log_info(format!("Detected Go version requirement: {requirement}"));

        let artifact = inv
            .resolve(&requirement)
            .ok_or(GoBuildpackError::VersionResolution(requirement))?;
        log_info(format!(
            "Resolved Go version: {}",
            artifact.semantic_version
        ));

        log_header("Installing Go distribution");
        let dist_layer = context.handle_layer(
            layer_name!("go_dist"),
            DistLayer {
                artifact: artifact.clone(),
            },
        )?;
        go_env = dist_layer.env.apply(Scope::Build, &go_env);

        log_header("Building Go packages");

        if Path::exists(&context.app_dir.join("vendor").join("modules.txt")) {
            log_info("Using vendored Go modules");
        } else {
            let deps_layer = context.handle_layer(layer_name!("go_deps"), DepsLayer {})?;
            go_env = deps_layer.env.apply(Scope::Build, &go_env);
        }

        let target_layer = context.handle_layer(layer_name!("go_target"), TargetLayer {})?;
        go_env = target_layer.env.apply(Scope::Build, &go_env);

        let build_layer = context.handle_layer(
            layer_name!("go_build"),
            BuildLayer {
                go_version: artifact.go_version.clone(),
            },
        )?;
        go_env = build_layer.env.apply(Scope::Build, &go_env);

        log_info("Resolving Go modules");
        let packages = cfg.packages.unwrap_or(
            // Use `go list` to determine packages to build. Do this eagerly,
            // even if the result is unused because it has the side effect of
            // downloading any required go modules.
            cmd::go_list(&go_env).map_err(GoBuildpackError::GoList)?,
        );

        log_info("Building packages:");
        for pkg in &packages {
            log_info(format!("  - {pkg}"));
        }
        cmd::go_install(&packages, &go_env).map_err(GoBuildpackError::GoBuild)?;

        log_header("Setting launch table");
        let procs = proc::build_procs(&packages).map_err(GoBuildpackError::Proc)?;
        log_info("Detected processes:");
        for proc in &procs {
            log_info(format!("  - {}: {}", proc.r#type, proc.command));
        }

        BuildResultBuilder::new()
            .launch(Launch::new().processes(procs))
            .build()
    }

    fn on_error(&self, error: libcnb::Error<Self::Error>) {
        match error {
            libcnb::Error::BuildpackError(bp_err) => {
                let err_string = bp_err.to_string();
                let err_ctx = match bp_err {
                    GoBuildpackError::BuildLayer(_) => "build layer",
                    GoBuildpackError::DepsLayer(_) => "dependency layer",
                    GoBuildpackError::DistLayer(_) => "distribution layer",
                    GoBuildpackError::TargetLayer(_) => "target layer",
                    GoBuildpackError::GoMod(_) => "go.mod",
                    GoBuildpackError::InventoryParse(_) => "inventory parse",
                    GoBuildpackError::VersionResolution(_) => "version resolution",
                    GoBuildpackError::GoBuild(_) => "go build",
                    GoBuildpackError::GoList(_) => "go list",
                    GoBuildpackError::Proc(_) => "launch process type",
                };
                log_error(format!("Heroku Go Buildpack {err_ctx} error"), err_string);
            }
            err => {
                log_error("Heroku Go Buildpack internal error", err.to_string());
            }
        }
    }
}

#[derive(Error, Debug)]
pub enum GoBuildpackError {
    #[error("{0}")]
    BuildLayer(#[from] BuildLayerError),
    #[error("Couldn't run `go build`: {0}")]
    GoBuild(CmdError),
    #[error("Couldn't run `go list`: {0}")]
    GoList(CmdError),
    #[error("Couldn't read go.mod build configuration: {0}")]
    GoMod(anyhow::Error),
    #[error("{0}")]
    DepsLayer(#[from] DepsLayerError),
    #[error("{0}")]
    DistLayer(#[from] DistLayerError),
    #[error("{0}")]
    TargetLayer(#[from] TargetLayerError),
    #[error("Couldn't parse go artifact inventory: {0}")]
    InventoryParse(toml::de::Error),
    #[error("Couldn't resolve go version for: {0}")]
    VersionResolution(Requirement),
    #[error("Launch process error: {0}")]
    Proc(proc::ProcErr),
}

impl From<GoBuildpackError> for libcnb::Error<GoBuildpackError> {
    fn from(e: GoBuildpackError) -> Self {
        libcnb::Error::BuildpackError(e)
    }
}

buildpack_main!(GoBuildpack);
