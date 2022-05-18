#![warn(unused_crate_dependencies)]
#![warn(clippy::pedantic)]
#![warn(clippy::cargo)]
#![allow(clippy::module_name_repetitions)]

use heroku_go_buildpack::inv::{
    list_github_go_versions, parse_go_semver, Artifact, Inventory, GITHUB_API_URL, GO_REPO_NAME,
};
use heroku_go_buildpack::ArtifactError;
use std::collections::HashSet;
use std::env;
use std::io::Error;
use std::process;

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

    for art in &mut inventory.artifacts {
        art.version = parse_go_semver(&art.go_version).unwrap_or_else(|e| {
            eprintln!("Error parsing goversion as semver: {e}");
            process::exit(4);
        });
    }

    let local_versions: HashSet<&str> = inventory
        .artifacts
        .iter()
        .map(|a| a.go_version.as_str())
        .collect();

    let remote_versions =
        list_github_go_versions(GITHUB_API_URL, GO_REPO_NAME).unwrap_or_else(|e| {
            eprintln!("Error listing go versions: {}", e);
            process::exit(4);
        });

    let new_versions: Vec<&str> = remote_versions
        .iter()
        .map(|rv| rv.as_str())
        .filter(|rv| !local_versions.contains(rv))
        .collect();

    let mut new_artifacts = vec![];
    for nv in &new_versions {
        match Artifact::new(nv.to_string()) {
            Ok(na) => {
                new_artifacts.push(na);
            }
            Err(err) => match err {
                ArtifactError::Checksum(e) => {
                    eprintln!("Error getting new go version checksum: {e}");
                }
                ArtifactError::SemVer(e) => {
                    eprintln!("Error parsing new go version: {e}");
                }
            },
        }
    }

    inventory.artifacts.append(&mut new_artifacts);

    inventory
        .artifacts
        .sort_by(|a, b| a.version.cmp(&b.version));

    let toml = toml::to_string(&inventory).unwrap_or_else(|e| {
        eprintln!("Error serializing inventory as toml: {e}");
        process::exit(5);
    });

    println!("{toml}");
}
