mod cfg;
mod cmd;
mod layers;
mod proc;
mod tgz;

use heroku_go_utils::vrs::GoVersion;
use layers::build::{BuildLayer, BuildLayerError};
use layers::deps::{handle_deps_layer, DepsLayerError};
use layers::dist::{handle_dist_layer, DistLayerError};
use layers::target::{TargetLayer, TargetLayerError};
use libcnb::build::{BuildContext, BuildResult, BuildResultBuilder};
use libcnb::data::build_plan::BuildPlanBuilder;
use libcnb::data::launch::{LaunchBuilder, Process};
use libcnb::data::layer_name;
use libcnb::detect::{DetectContext, DetectResult, DetectResultBuilder};
use libcnb::generic::GenericMetadata;
use libcnb::generic::GenericPlatform;
use libcnb::layer_env::Scope;
use libcnb::{buildpack_main, Buildpack, Env};
use libherokubuildpack::inventory::artifact::{Arch, Os};
use libherokubuildpack::inventory::Inventory;
use libherokubuildpack::log::{log_error, log_header, log_info};
use sha2::Sha256;
use std::env::{self, consts};
use std::path::Path;

#[cfg(test)]
use libcnb_test as _;

const INVENTORY: &str = include_str!("../inventory.toml");

struct GoBuildpack;

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

        let inv: Inventory<GoVersion, Sha256, Option<()>> =
            toml::from_str(INVENTORY).map_err(GoBuildpackError::InventoryParse)?;

        let config = cfg::read_gomod_config(context.app_dir.join("go.mod"))
            .map_err(GoBuildpackError::GoModConfig)?;
        let requirement = config.version.unwrap_or_default();
        log_info(format!("Detected Go version requirement: {requirement}"));

        let artifact = match (consts::OS.parse::<Os>(), consts::ARCH.parse::<Arch>()) {
            (Ok(os), Ok(arch)) => inv.resolve(os, arch, &requirement),
            (_, _) => None,
        }
        .ok_or(GoBuildpackError::VersionResolution(requirement.clone()))?;

        log_info(format!(
            "Resolved Go version: {} ({}-{})",
            artifact.version, artifact.os, artifact.arch
        ));

        log_header("Installing Go distribution");
        go_env = handle_dist_layer(&context, artifact)?
            .read_env()?
            .apply(Scope::Build, &go_env);

        log_header("Building Go binaries");

        if Path::exists(&context.app_dir.join("vendor").join("modules.txt")) {
            log_info("Using vendored Go modules");
        } else {
            go_env = handle_deps_layer(&context)?.apply(Scope::Build, &go_env);
        }

        go_env = context
            .handle_layer(layer_name!("go_target"), TargetLayer {})?
            .env
            .apply(Scope::Build, &go_env);

        go_env = context
            .handle_layer(
                layer_name!("go_build"),
                BuildLayer {
                    go_version: artifact.version.clone(),
                },
            )?
            .env
            .apply(Scope::Build, &go_env);

        log_info("Resolving Go modules");
        let packages = config.packages.unwrap_or(
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

        let mut procs: Vec<Process> = vec![];
        if Path::exists(&context.app_dir.join("Procfile")) {
            log_info("Skipping launch process registration (Procfile detected)");
        } else {
            log_header("Registering launch processes");
            procs = proc::build_procs(&packages).map_err(GoBuildpackError::Proc)?;
            log_info("Detected processes:");
            for proc in &procs {
                log_info(format!("  - {}: {}", proc.r#type, proc.command.join(" ")));
            }
        }

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
enum GoBuildpackError {
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
    VersionResolution(semver::VersionReq),
    #[error("Launch process error: {0}")]
    Proc(proc::Error),
}

impl From<GoBuildpackError> for libcnb::Error<GoBuildpackError> {
    fn from(e: GoBuildpackError) -> Self {
        libcnb::Error::BuildpackError(e)
    }
}

buildpack_main!(GoBuildpack);
