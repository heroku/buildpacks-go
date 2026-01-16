mod inv;

use crate::inv::list_upstream_artifacts;
use heroku_go_utils::vrs::GoVersion;
use keep_a_changelog_file::{ChangeGroup, Changelog};
use libherokubuildpack::inventory::{Inventory, artifact::Artifact};
use sha2::Sha256;
use std::{env, fs, process, str::FromStr};

/// Updates the local go inventory.toml with versions published on go.dev.
fn main() {
    let (inventory_path, changelog_path) = {
        let args: Vec<String> = env::args().collect();
        if args.len() != 3 {
            eprintln!("Usage: inventory-updater <path/to/inventory.toml> <path/to/CHANGELOG.md>");
            process::exit(2);
        }
        (args[1].clone(), args[2].clone())
    };

    let old_inventory = fs::read_to_string(&inventory_path)
        .unwrap_or_else(|e| {
            eprintln!("Error reading inventory at '{inventory_path}': {e}");
            std::process::exit(1);
        })
        .parse::<Inventory<GoVersion, Sha256, Option<()>>>()
        .unwrap_or_else(|e| {
            eprintln!("Error parsing inventory file at '{inventory_path}': {e}");
            process::exit(1);
        });

    // List available upstream release versions.
    let remote_artifacts = list_upstream_artifacts().unwrap_or_else(|e| {
        eprintln!("Failed to fetch upstream go versions: {e}");
        process::exit(4);
    });

    let new_inventory = Inventory {
        artifacts: remote_artifacts,
    };

    let toml = toml::to_string(&new_inventory).unwrap_or_else(|e| {
        eprintln!("Error serializing inventory as toml: {e}");
        process::exit(6);
    });

    fs::write(&inventory_path, toml).unwrap_or_else(|e| {
        eprintln!("Error writing inventory to file: {e}");
        process::exit(7);
    });

    let changelog_contents = fs::read_to_string(&changelog_path).unwrap_or_else(|e| {
        eprintln!("Error reading changelog at '{changelog_path}': {e}");
        process::exit(8);
    });

    let mut changelog = Changelog::from_str(&changelog_contents).unwrap_or_else(|e| {
        eprintln!("Error parsing changelog at '{changelog_path}': {e}");
        process::exit(9);
    });

    update_changelog(
        &mut changelog,
        &ChangeGroup::Added,
        &difference(&new_inventory.artifacts, &old_inventory.artifacts),
    );
    update_changelog(
        &mut changelog,
        &ChangeGroup::Removed,
        &difference(&old_inventory.artifacts, &new_inventory.artifacts),
    );

    fs::write(&changelog_path, changelog.to_string()).unwrap_or_else(|e| {
        eprintln!("Failed to write to changelog: {e}");
        process::exit(10);
    });
}

/// Finds the difference between two slices.
fn difference<'a, T: Eq>(a: &'a [T], b: &'a [T]) -> Vec<&'a T> {
    a.iter().filter(|&artifact| !b.contains(artifact)).collect()
}

/// Helper function to update the changelog.
fn update_changelog(
    changelog: &mut Changelog,
    change_group: &ChangeGroup,
    artifacts: &[&Artifact<GoVersion, Sha256, Option<()>>],
) {
    if !artifacts.is_empty() {
        let mut versions: Vec<_> = artifacts.iter().map(|artifact| &artifact.version).collect();
        versions.sort_unstable();
        versions.dedup();
        for version in versions {
            changelog
                .unreleased
                .add(change_group.clone(), format!("Support for {version}."));
        }
    }
}
