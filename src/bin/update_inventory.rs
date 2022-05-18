#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

use heroku_go_buildpack::inv::{list_github_go_versions, Artifact, Inventory};
use heroku_go_buildpack::ArtifactError;
use std::collections::HashSet;
use std::{env, fs, process};

/// Updates the local go inventory.toml with versions published on GitHub.
fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: update_inventory <path/to/inventory.toml>");
        process::exit(2);
    }

    let filename = &args[1];

    let mut inventory = Inventory::read(filename).unwrap_or_else(|e| {
        eprintln!("Error reading inventory '{}': {}", filename, e);
        process::exit(3);
    });

    let local_versions: HashSet<&str> = inventory
        .artifacts
        .iter()
        .map(|a| a.go_version.as_str())
        .collect();

    // List available versions published to GitHub.
    let remote_versions = list_github_go_versions().unwrap_or_else(|e| {
        eprintln!("Error listing go versions: {}", e);
        process::exit(4);
    });

    // Find versions from GitHub that are not in the local inventory.
    let new_versions: Vec<&str> = remote_versions
        .iter()
        .map(|rv| rv.as_str())
        .filter(|rv| !local_versions.contains(rv))
        .collect();

    // Build new artifacts for the GitHub releases we don't have yet.
    let mut new_artifacts = vec![];
    for nv in &new_versions {
        match Artifact::new(nv.to_string()) {
            Ok(na) => {
                new_artifacts.push(na);
            }
            Err(err) => match err {
                ArtifactError::Checksum(e) => {
                    // Some older versions of go don't seem to have published sha256 files.
                    eprintln!("Error getting new go version checksum: {e}");
                }
                ArtifactError::Version(e) => {
                    eprintln!("Error parsing new go version: {e}");
                    process::exit(5);
                }
            },
        }
    }

    // Concatenate the existing and new artifacts.
    inventory.artifacts.append(&mut new_artifacts);

    // Sort artifacts in reverse semver order, to make it easier to resolve
    // to the most recent version for a semver constraint.
    inventory
        .artifacts
        .sort_by(|b, a| a.semantic_version.cmp(&b.semantic_version));

    let toml = toml::to_string(&inventory).unwrap_or_else(|e| {
        eprintln!("Error serializing inventory as toml: {e}");
        process::exit(6);
    });

    fs::write(filename, toml).unwrap_or_else(|e| {
        eprintln!("Error writing inventory to file: {e}");
        process::exit(7);
    });
}
