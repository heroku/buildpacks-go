mod cfg;
mod cmd;
mod layers;
mod proc;
mod tgz;

use bullet_stream::global::print;
use bullet_stream::{style, Print};
use fs_err::PathExt;
use heroku_go_utils::vrs::GoVersion;
use layers::build::BuildLayerError;
use layers::deps::DepsLayerError;
use layers::dist::DistLayerError;
use libcnb::build::{BuildContext, BuildResult, BuildResultBuilder};
use libcnb::data::build_plan::BuildPlanBuilder;
use libcnb::data::launch::LaunchBuilder;
use libcnb::data::layer_name;
use libcnb::detect::{DetectContext, DetectResult, DetectResultBuilder};
use libcnb::generic::GenericMetadata;
use libcnb::generic::GenericPlatform;
use libcnb::layer::UncachedLayerDefinition;
use libcnb::layer_env::{LayerEnv, Scope};
use libcnb::{buildpack_main, Buildpack, Env};
use libherokubuildpack::inventory::artifact::{Arch, Os};
use libherokubuildpack::inventory::Inventory;
use sha2::Sha256;
use std::env::{self, consts};
use std::path::Path;
use std::time::Instant;

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
        let started = Instant::now();
        let mut go_env = Env::new();
        env::vars()
            .filter(|(k, _v)| k == "PATH")
            .for_each(|(k, v)| {
                go_env.insert(k, v);
            });

        let mut bullet = build_output.bullet("Go version");
        let inv: Inventory<GoVersion, Sha256, Option<()>> =
            toml::from_str(INVENTORY).map_err(GoBuildpackError::InventoryParse)?;
        let go_mod = context.app_dir.join("go.mod");
        let config = cfg::read_gomod_config(&go_mod).map_err(GoBuildpackError::GoModConfig)?;
        let requirement = config.version.unwrap_or_default();

        bullet = bullet.sub_bullet(format!(
            "Detected requirement {req} (from {file})",
            req = style::value(requirement.to_string()),
            file = go_mod.display()
        ));

        let artifact = match (consts::OS.parse::<Os>(), consts::ARCH.parse::<Arch>()) {
            (Ok(os), Ok(arch)) => inv.resolve(os, arch, &requirement),
            (_, _) => None,
        }
        .ok_or(GoBuildpackError::VersionResolution(requirement.clone()))?;

        bullet = bullet.sub_bullet(format!(
            "Resolved to {}",
            style::value(artifact.version.to_string()),
        ));

        (build_output, go_env) = {
            layers::dist::call(&context, bullet, &layers::dist::Metadata::new(artifact)).map(
                |(bullet, layer_env)| (bullet.done(), layer_env.apply(Scope::Build, &go_env)),
            )?
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
                    bullet.sub_bullet("Using vendored Go modules").done(),
                    go_env,
                )
            } else {
                layers::deps::call(&context, bullet, &layers::deps::Metadata::new(1.0)).map(
                    |(bullet, layer_env)| (bullet.done(), layer_env.apply(Scope::Build, &go_env)),
                )?
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
                &layers::build::Metadata::new(&artifact.version, &context.target),
            )
            .map(|(bullet, layer_env)| (bullet.done(), layer_env.apply(Scope::Build, &go_env)))?
        };

        let bullet = build_output.bullet("Go module resolution");
        let (bullet, packages) = if let Some(packages) = config.packages {
            (bullet.sub_bullet("Found packages in go.mod"), packages)
        } else {
            cmd::go_list(bullet, &go_env).map_err(GoBuildpackError::GoList)?
        };

        let mut bullet = bullet.done().bullet("Packages found");
        for pkg in &packages {
            bullet = bullet.sub_bullet(style::value(pkg));
        }

        print::bullet("Go install");
        cmd::go_install(&packages, &go_env).map_err(GoBuildpackError::GoBuild)?;

        print::bullet("Default processes");
        let procs = if Path::exists(&context.app_dir.join("Procfile")) {
            print::sub_bullet("Skipping (Procfile detected)");
            Vec::new()
        } else {
            let procs = proc::build_procs(&packages).map_err(GoBuildpackError::Proc)?;
            if procs.is_empty() {
                print::sub_bullet("No processes found");
            } else {
                for proc in &procs {
                    print::sub_bullet(format!(
                        "{}: {}",
                        proc.r#type,
                        style::command(proc.command.join(" "))
                    ));
                }
            }
            procs
        };

        print::all_done(&Some(started));

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
                print::error(format!(
                    "Heroku Go Buildpack {err_ctx} error\n\n{err_string}"
                ));
            }
            err => print::error(format!("Heroku Go Buildpack internal error\n\n{err}")),
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
