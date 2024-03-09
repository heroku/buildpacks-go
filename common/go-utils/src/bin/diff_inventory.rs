// Required due to: https://github.com/rust-lang/rust/issues/95513
#![allow(unused_crate_dependencies)]

use heroku_go_utils::inv::{list_upstream_artifacts, Artifact, Inventory};
use std::collections::HashSet;

/// Prints a human-readable software inventory difference. Useful
/// for generating commit messages and changelogs for automated inventory
/// updates.
fn main() {
    let inventory_path = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("$ diff_inventory path/to/inventory.toml");
        std::process::exit(1);
    });

    let upstream_artifacts: HashSet<Artifact> = list_upstream_artifacts()
        .unwrap_or_else(|e| {
            eprintln!("Failed to fetch upstream go versions: {e}");
            std::process::exit(1)
        })
        .into_iter()
        .collect();

    let inventory_artifacts: HashSet<Artifact> = Inventory::read(&inventory_path)
        .unwrap_or_else(|e| {
            eprintln!("Error reading inventory at '{inventory_path}': {e}");
            std::process::exit(1);
        })
        .artifacts
        .into_iter()
        .collect();

    let mut added_artifacts: Vec<&Artifact> = upstream_artifacts
        .difference(&inventory_artifacts)
        .collect();

    added_artifacts.sort_by_cached_key(|a| &a.semantic_version);

    if !added_artifacts.is_empty() {
        println!(
            "Added {}.",
            added_artifacts
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    let mut removed_artifacts: Vec<&Artifact> = inventory_artifacts
        .difference(&upstream_artifacts)
        .collect();

    removed_artifacts.sort_by_cached_key(|a| &a.semantic_version);

    if !removed_artifacts.is_empty() {
        println!(
            "Removed {}.",
            removed_artifacts
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
}
