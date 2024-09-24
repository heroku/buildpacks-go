// Required due to: https://github.com/rust-lang/rust/issues/95513
#![allow(unused_crate_dependencies)]

use heroku_go_utils::{inv::list_upstream_artifacts, vrs::GoVersion};
use libherokubuildpack::inventory::{artifact::Artifact, Inventory};
use sha2::Sha256;
use std::{env, fs, process};

/// Updates the local go inventory.toml with versions published on go.dev.
fn main() {
    let inventory_path = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: update_inventory <path/to/inventory.toml>");
        process::exit(2);
    });

    let inventory_artifacts = fs::read_to_string(&inventory_path)
        .unwrap_or_else(|e| {
            eprintln!("Error reading inventory at '{inventory_path}': {e}");
            std::process::exit(1);
        })
        .parse::<Inventory<GoVersion, Sha256, Option<()>>>()
        .unwrap_or_else(|e| {
            eprintln!("Error parsing inventory file at '{inventory_path}': {e}");
            process::exit(1);
        })
        .artifacts;

    // List available upstream release versions.
    let remote_artifacts = list_upstream_artifacts().unwrap_or_else(|e| {
        eprintln!("Failed to fetch upstream go versions: {e}");
        process::exit(4);
    });

    let inventory = Inventory {
        artifacts: remote_artifacts,
    };

    let toml = toml::to_string(&inventory).unwrap_or_else(|e| {
        eprintln!("Error serializing inventory as toml: {e}");
        process::exit(6);
    });

    fs::write(inventory_path, toml).unwrap_or_else(|e| {
        eprintln!("Error writing inventory to file: {e}");
        process::exit(7);
    });

    let remote_artifacts: Vec<Artifact<GoVersion, Sha256, Option<()>>> =
        inventory.artifacts.into_iter().collect();

    [
        ("Added", difference(&remote_artifacts, &inventory_artifacts)),
        (
            "Removed",
            difference(&inventory_artifacts, &remote_artifacts),
        ),
    ]
    .iter()
    .filter(|(_, artifact_diff)| !artifact_diff.is_empty())
    .for_each(|(action, artifacts)| {
        let mut list: Vec<_> = artifacts.iter().collect();
        list.sort_by_key(|a| &a.version);
        println!(
            "{} {}.",
            action,
            list.iter()
                .map(|artifact| format!("{} ({}-{})", artifact.version, artifact.os, artifact.arch))
                .collect::<Vec<_>>()
                .join(", ")
        );
    });
}

/// Finds the difference between two slices.
fn difference<'a, T: Eq>(a: &'a [T], b: &'a [T]) -> Vec<&'a T> {
    a.iter().filter(|&artifact| !b.contains(artifact)).collect()
}
