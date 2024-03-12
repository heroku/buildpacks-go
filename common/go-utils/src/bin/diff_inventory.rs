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

    [
        ("Added", &upstream_artifacts - &inventory_artifacts),
        ("Removed", &inventory_artifacts - &upstream_artifacts),
    ]
    .iter()
    .filter(|(_, artifact_diff)| !artifact_diff.is_empty())
    .for_each(|(action, artifacts)| {
        let mut list: Vec<&Artifact> = artifacts.iter().collect();
        list.sort_by_key(|a| &a.semantic_version);
        println!(
            "{} {}.",
            action,
            list.iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        );
    });
}
