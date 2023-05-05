#![warn(clippy::pedantic)]
#![warn(clippy::cargo)]

mod cfg;
mod cmd;
mod layers;
mod log;
mod proc;
mod tgz;

use heroku_go_buildpack::inv::Inventory;
use heroku_go_buildpack::vrs::Requirement;
use layers::build::{BuildLayer, BuildLayerError};
use layers::deps::{DepsLayer, DepsLayerError};
use layers::dist::{DistLayer, DistLayerError};
use layers::target::{TargetLayer, TargetLayerError};
use libcnb::build::{BuildContext, BuildResult, BuildResultBuilder};
use libcnb::data::build_plan::BuildPlanBuilder;
use libcnb::data::launch::LaunchBuilder;
use libcnb::data::layer_name;
use libcnb::detect::{DetectContext, DetectResult, DetectResultBuilder};
use libcnb::generic::GenericMetadata;
use libcnb::generic::GenericPlatform;
use libcnb::layer_env::Scope;
use libcnb::{buildpack_main, Buildpack, Env};
use libherokubuildpack::log::{log_error, log_header, log_info};
use std::env;
use std::path::Path;
use termcolor::{ColorChoice, StandardStream};

const INVENTORY: &str = include_str!("../inventory.toml");

pub(crate) struct GoBuildpack;

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
        let mut log = log::Logger::new(StandardStream::stderr(ColorChoice::Always));
        let mut go_env = Env::new();
        env::vars()
            .filter(|(k, _v)| k == "PATH")
            .for_each(|(k, v)| {
                go_env.insert(k, v);
            });

        log.header("Reading build configuration");
        let inv: Inventory = toml::from_str(INVENTORY).map_err(GoBuildpackError::InventoryParse)?;

        let config = cfg::read_gomod_config(context.app_dir.join("go.mod"))
            .map_err(GoBuildpackError::GoModConfig)?;
        let requirement = config.version.unwrap_or_default();
        log.info(format!("Detected Go version requirement: {requirement}"));

        let artifact = inv
            .resolve(&requirement)
            .ok_or(GoBuildpackError::VersionResolution(requirement))?;
        log.info(format!(
            "Resolved Go version: {}",
            artifact.semantic_version
        ));

        let mut go_env = log
            .with_block("Installing Go distribution", |log| {
                context.handle_layer(
                    layer_name!("go_dist"),
                    DistLayer {
                        artifact: artifact.clone(),
                        log: *log,
                    },
                )
            })?
            .env
            .apply(Scope::Build, &go_env);

        log.header("Building Go binaries");

        if Path::exists(&context.app_dir.join("vendor").join("modules.txt")) {
            log.info("Using vendored Go modules");
        } else {
            go_env = context
                .handle_layer(
                    layer_name!("go_deps"),
                    DepsLayer {
                        go_env: go_env.clone(),
                    },
                )?
                .env
                .apply(Scope::Build, &go_env);
        }

        go_env = context
            .handle_layer(layer_name!("go_target"), TargetLayer {})?
            .env
            .apply(Scope::Build, &go_env);

        go_env = context
            .handle_layer(
                layer_name!("go_build"),
                BuildLayer {
                    go_version: artifact.go_version.clone(),
                },
            )?
            .env
            .apply(Scope::Build, &go_env);

        log.info("Resolving Go modules");
        let packages = config.packages.unwrap_or(
            // Use `go list` to determine packages to build. Do this eagerly,
            // even if the result is unused because it has the side effect of
            // downloading any required go modules.
            cmd::go_list(&go_env).map_err(GoBuildpackError::GoList)?,
        );

        log.with_block("Building packages", |log| {
            for pkg in &packages {
                log.info(pkg);
            }
            cmd::go_install(&packages, &go_env).map_err(GoBuildpackError::GoBuild)
        })?;

        let procs = log.with_block("Setting launch table", |launch_log| {
            proc::build_procs(&packages)
                .map_err(GoBuildpackError::Proc)
                .map(|procs| {
                    launch_log.info("Detected processes:");
                    for proc in &procs {
                        launch_log.info(format!("- {}: {}", proc.r#type, proc.command));
                    }
                    procs
                })
        })?;

        BuildResultBuilder::new()
            .launch(LaunchBuilder::new().processes(procs).build())
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
                    GoBuildpackError::GoModConfig(_) => "go.mod",
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

#[derive(thiserror::Error, Debug)]
pub(crate) enum GoBuildpackError {
    #[error("{0}")]
    BuildLayer(#[from] BuildLayerError),
    #[error("Couldn't run `go build`: {0}")]
    GoBuild(cmd::Error),
    #[error("Couldn't run `go list`: {0}")]
    GoList(cmd::Error),
    #[error("{0}")]
    GoModConfig(#[from] cfg::ReadGoModConfigError),
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
    Proc(proc::Error),
}

impl From<GoBuildpackError> for libcnb::Error<GoBuildpackError> {
    fn from(e: GoBuildpackError) -> Self {
        libcnb::Error::BuildpackError(e)
    }
}

buildpack_main!(GoBuildpack);
