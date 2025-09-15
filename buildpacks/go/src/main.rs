// cargo-llvm-cov sets the coverage_nightly attribute when instrumenting our code. In that case,
// we enable https://doc.rust-lang.org/beta/unstable-book/language-features/coverage-attribute.html
// to be able selectively opt out of coverage for functions/lines/modules.
#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

mod cfg;
mod cmd;
mod layers;
mod proc;
mod tgz;

use bullet_stream::global::print;
use bullet_stream::style;
use heroku_go_utils::vrs::GoVersion;
use indoc::formatdoc;
use layers::build::{BuildLayerError, handle_build_layer};
use layers::deps::{DepsLayerError, handle_deps_layer};
use layers::dist::{DistLayerError, handle_dist_layer};
use layers::target::{TargetLayerError, handle_target_layer};
use libcnb::build::{BuildContext, BuildResult, BuildResultBuilder};
use libcnb::data::build_plan::BuildPlanBuilder;
use libcnb::data::launch::{LaunchBuilder, Process};
use libcnb::detect::{DetectContext, DetectResult, DetectResultBuilder};
use libcnb::generic::GenericMetadata;
use libcnb::generic::GenericPlatform;
use libcnb::layer_env::Scope;
use libcnb::{Buildpack, Env, buildpack_main};
use libherokubuildpack::inventory::Inventory;
use libherokubuildpack::inventory::artifact::{Arch, Os};
use sha2::Sha256;
use std::env::consts;
use std::path::Path;
use std::time::Instant;

#[cfg(test)]
use libcnb_test as _;

#[cfg(test)]
use indoc as _;

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
        print::h2("Heroku Go Buildpack");
        print::bullet("Reading build configuration");

        let started = Instant::now();
        let mut go_env = Env::from_current();
        let go_prefixed_envs = go_env
            .iter()
            .map(|(key, _)| key.to_string_lossy())
            .filter(|key| key.starts_with("GO"))
            .map(bullet_stream::style::value)
            .collect::<Vec<_>>()
            .join(", ");

        if !go_prefixed_envs.is_empty() {
            print::warning(formatdoc! {"
                WARNING: Found `GO` prefixed environment variables not set by this buildpack.
                These variables may impact build or runtime behavior in unexpected ways.

                Environment variables: {go_prefixed_envs}
            "});
        }

        let inv: Inventory<GoVersion, Sha256, Option<()>> =
            toml::from_str(INVENTORY).map_err(GoBuildpackError::InventoryParse)?;

        let config = cfg::read_gomod_config(context.app_dir.join("go.mod"))
            .map_err(GoBuildpackError::GoModConfig)?;
        let requirement = config.version.unwrap_or_default();
        print::sub_bullet(format!("Detected Go version requirement: {requirement}"));

        let artifact = match (consts::OS.parse::<Os>(), consts::ARCH.parse::<Arch>()) {
            (Ok(os), Ok(arch)) => inv.resolve(os, arch, &requirement),
            (_, _) => None,
        }
        .ok_or(GoBuildpackError::VersionResolution(requirement.clone()))?;

        print::sub_bullet(format!(
            "Resolved Go version: {} ({}-{})",
            artifact.version, artifact.os, artifact.arch
        ));

        print::bullet("Installing Go distribution");
        go_env = handle_dist_layer(&context, artifact)?
            .read_env()?
            .apply(Scope::Build, &go_env);

        print::bullet("Building Go binaries");
        if Path::exists(&context.app_dir.join("vendor").join("modules.txt")) {
            print::sub_bullet("Using vendored Go modules");
        } else {
            go_env = handle_deps_layer(&context)?.apply(Scope::Build, &go_env);
        }

        go_env = handle_target_layer(&context)?.apply(Scope::Build, &go_env);

        go_env = handle_build_layer(&context, &artifact.version)?.apply(Scope::Build, &go_env);

        print::sub_bullet("Resolving Go modules");
        let packages = config.packages.unwrap_or(
            // Use `go list` to determine packages to build. Do this eagerly,
            // even if the result is unused because it has the side effect of
            // downloading any required go modules.
            cmd::go_list(&go_env).map_err(GoBuildpackError::GoList)?,
        );

        print::bullet("Building packages:");
        for pkg in &packages {
            print::sub_bullet(pkg);
        }
        cmd::go_install(&packages, &go_env).map_err(GoBuildpackError::GoBuild)?;

        let mut procs: Vec<Process> = vec![];
        if Path::exists(&context.app_dir.join("Procfile")) {
            print::bullet("Skipping launch process registration (Procfile detected)");
        } else {
            print::bullet("Registering launch processes:");
            procs = proc::build_procs(&packages).map_err(GoBuildpackError::Proc)?;
            for proc in &procs {
                print::sub_bullet(format!(
                    "{}: {}",
                    proc.r#type,
                    style::command(proc.command.join(" "))
                ));
            }
        }

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
                };
                print::error(format!(
                    "Heroku Go Buildpack {err_ctx} error\n\n{err_string}"
                ));
            }
            err => {
                print::error(format!("Heroku Go Buildpack internal error\n\n{err}"));
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
