mod cfg;
mod cmd;
mod layers;
mod proc;
mod tgz;

use bullet_stream::{style, Print};
use fs_err::PathExt;
use heroku_go_utils::vrs::GoVersion;
use layers::build::BuildLayerError;
use layers::deps::DepsLayerError;
use layers::dist::DistLayerError;
use libcnb::build::{BuildContext, BuildResult, BuildResultBuilder};
use libcnb::data::build_plan::BuildPlanBuilder;
use libcnb::data::launch::{LaunchBuilder, Process};
use libcnb::data::layer_name;
use libcnb::detect::{DetectContext, DetectResult, DetectResultBuilder};
use libcnb::generic::GenericMetadata;
use libcnb::generic::GenericPlatform;
use libcnb::layer::UncachedLayerDefinition;
use libcnb::layer_env::{LayerEnv, Scope};
use libcnb::{buildpack_main, Buildpack, Env};
use libherokubuildpack::inventory::artifact::{Arch, Os};
use libherokubuildpack::inventory::Inventory;
use libherokubuildpack::log::{log_error, log_info};
use sha2::Sha256;
use std::env::{self, consts};
use std::path::Path;

#[cfg(test)]
use libcnb_test as _;
use serde_json as _;

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

    #[allow(clippy::too_many_lines)]
    fn build(&self, context: BuildContext<Self>) -> libcnb::Result<BuildResult, Self::Error> {
        let mut build_output = Print::global().h2("Heroku Go Buildpack");
        let mut go_env = Env::new();
        env::vars()
            .filter(|(k, _v)| k == "PATH")
            .for_each(|(k, v)| {
                go_env.insert(k, v);
            });

        let bullet = build_output.bullet("Build configuration");
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

        build_output = bullet
            .sub_bullet(format!(
                "Resolved Go version: {} ({}-{})",
                style::value(artifact.version.to_string()),
                artifact.os,
                artifact.arch
            ))
            .done();

        (build_output, go_env) = {
            layers::dist::call(
                &context,
                build_output.bullet("Installing Go distribution"),
                &layers::dist::DistLayerMetadata::new(artifact),
            )
            .map(|(bullet, layer_env)| (bullet.done(), layer_env.apply(Scope::Build, &go_env)))?
        };

        (build_output, go_env) = {
            let bullet = build_output.bullet("Go binaries");
            if context
                .app_dir
                .join("vendor")
                .join("modules.txt")
                .fs_err_try_exists()
                .map_err(GoBuildpackError::FsTryExist)?
            {
                (
                    bullet.sub_bullet("Detected vendored Go modules").done(),
                    go_env,
                )
            } else {
                layers::deps::call(&context, bullet, &layers::deps::DepsLayerMetadata::new(1.0))
                    .map(|(bullet, layer_env)| {
                        (bullet.done(), layer_env.apply(Scope::Build, &go_env))
                    })?
            }
        };

        (build_output, go_env) = {
            let layer_ref = context.uncached_layer(
                layer_name!("go_target"),
                UncachedLayerDefinition {
                    build: true,
                    launch: true,
                },
            )?;

            fs_err::create_dir(layer_ref.path().join("bin"))
                .map_err(GoBuildpackError::TargetLayer)?;
            layer_ref.write_env(LayerEnv::new().chainable_insert(
                Scope::Build,
                libcnb::layer_env::ModificationBehavior::Override,
                "GOBIN",
                layer_ref.path().join("bin"),
            ))?;
            (
                build_output,
                layer_ref.read_env()?.apply(Scope::Build, &go_env),
            )
        };
        (build_output, go_env) = {
            layers::build::call(
                &context,
                build_output.bullet("Go build cache"),
                &layers::build::BuildLayerMetadata::new(&artifact.version, &context.target),
            )
            .map(|(bullet, layer_env)| (bullet.done(), layer_env.apply(Scope::Build, &go_env)))?
        };

        log_info("Resolving Go modules");
        let packages = config.packages.unwrap_or(
            // Use `go list` to determine packages to build. Do this eagerly,
            // even if the result is unused because it has the side effect of
            // downloading any required go modules.
            cmd::go_list(&go_env).map_err(GoBuildpackError::GoList)?,
        );

        build_output = {
            let mut bullet = build_output.bullet("Packages found");
            for pkg in &packages {
                bullet = bullet.sub_bullet(format!("  - {pkg}"));
            }
            bullet = bullet.done().bullet("Go install");
            cmd::go_install(&packages, &go_env).map_err(GoBuildpackError::GoBuild)?;
            bullet.done()
        };

        let (build_output, procs) = {
            let mut bullet = build_output.bullet("Default processes");

            let mut procs: Vec<Process> = vec![];
            if Path::exists(&context.app_dir.join("Procfile")) {
                bullet = bullet.sub_bullet("Skipping (Procfile detected)");
            } else {
                procs = proc::build_procs(&packages).map_err(GoBuildpackError::Proc)?;
                if procs.is_empty() {
                    bullet = bullet.sub_bullet("No processes found");
                } else {
                    for proc in &procs {
                        bullet = bullet.sub_bullet(format!(
                            "{}: {}",
                            proc.r#type,
                            style::command(proc.command.join(" "))
                        ));
                    }
                }
            }

            (bullet.done(), procs)
        };
        build_output.done();

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
                    GoBuildpackError::FsTryExist(_) => "file system",
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
    TargetLayer(std::io::Error),
    #[error("Couldn't parse go artifact inventory: {0}")]
    InventoryParse(toml::de::Error),
    #[error("Couldn't resolve go version for: {0}")]
    VersionResolution(semver::VersionReq),
    #[error("Launch process error: {0}")]
    Proc(proc::Error),
    #[error("Could not access file system due to error: {0}")]
    FsTryExist(std::io::Error),
}

impl From<GoBuildpackError> for libcnb::Error<GoBuildpackError> {
    fn from(e: GoBuildpackError) -> Self {
        libcnb::Error::BuildpackError(e)
    }
}

buildpack_main!(GoBuildpack);
