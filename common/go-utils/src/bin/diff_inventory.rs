// Required due to: https://github.com/rust-lang/rust/issues/95513
#![allow(unused_crate_dependencies)]

use heroku_go_utils::inv::{list_upstream_artifacts, Inventory};
use std::collections::HashSet;

/// Prints a human-readable software inventory difference. Useful
/// for generating commit messages and changelogs for automated inventory
/// updates.
fn main() {
    let inventory_path = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("$ diff_inventory path/to/inventory.toml");
        std::process::exit(1);
    });

    let upstream_versions: HashSet<String> = list_upstream_artifacts()
        .unwrap_or_else(|e| {
            eprintln!("Failed to fetch upstream go versions: {e}");
            std::process::exit(1)
        })
        .into_iter()
        .map(|a| a.go_version)
        .collect();

    let local_versions: HashSet<String> = Inventory::read(&inventory_path)
        .unwrap_or_else(|e| {
            eprintln!("Error reading inventory at '{inventory_path}': {e}");
            std::process::exit(1);
        })
        .artifacts
        .iter()
        .map(|r| r.go_version.to_string())
        .collect();

    let mut added_versions: Vec<String> = upstream_versions
        .difference(&local_versions)
        .map(String::to_string)
        .collect();

    added_versions.sort();

    if !added_versions.is_empty() {
        println!("Added {}.", added_versions.join(", "));
    }

    let mut removed_versions: Vec<String> = local_versions
        .difference(&upstream_versions)
        .map(String::to_string)
        .collect();

    removed_versions.sort();

    if !removed_versions.is_empty() {
        println!("Removed {}.", removed_versions.join(", "));
    }
}
