use crate::inv::{Artifact, Inventory};
use crate::vrs::Version;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashSet;
use std::fmt::Display;
use std::{env, fs, process};

// Todo: Refactor with a different name
#[allow(clippy::module_name_repetitions)]
pub trait UpstreamInventory<V>
where
    V: Version + DeserializeOwned + Serialize + Clone,
{
    type Error: Display;

    /// # Errors
    ///
    /// Issues listing upstream artifacts will return an Error
    fn list_upstream_artifacts() -> Result<HashSet<Artifact<V>>, Self::Error>;

    /// Updates a local inventory.toml with versions published upstream.
    fn update_local() {
        let path = inventory_path();

        let mut remote_artifacts: Vec<Artifact<V>> = Self::list_upstream_artifacts()
            .unwrap_or_else(|e| {
                eprintln!("Failed to fetch upstream artifacts: {e}");
                process::exit(4)
            })
            .into_iter()
            .collect();

        remote_artifacts.sort();
        remote_artifacts.reverse();

        let inventory = Inventory {
            artifacts: remote_artifacts,
        };

        let toml = toml::to_string(&inventory).unwrap_or_else(|e| {
            eprintln!("Error serializing inventory as toml: {e}");
            process::exit(6);
        });

        fs::write(path, toml).unwrap_or_else(|e| {
            eprintln!("Error writing inventory to file: {e}");
            process::exit(7);
        });
    }

    /// Prints a human-readable inventory diff. Useful for generating
    /// commit messages and changelogs for automated inventory updates.
    fn diff_inventory() {
        let path = std::env::args().nth(1).unwrap_or_else(|| {
            eprintln!("$ diff_inventory path/to/inventory.toml");
            std::process::exit(1);
        });

        let upstream_artifacts: HashSet<Artifact<V>> = Self::list_upstream_artifacts()
            .unwrap_or_else(|e| {
                eprintln!("Failed to fetch upstream artifacts: {e}");
                std::process::exit(1)
            });

        let inventory_artifacts: HashSet<Artifact<V>> = Inventory::read(&path)
            .unwrap_or_else(|e| {
                eprintln!("Error reading inventory at '{path}': {e}");
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
            let mut list: Vec<&Artifact<V>> = artifacts.iter().collect();
            list.sort();
            list.reverse();
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
}

fn inventory_path() -> String {
    env::args().nth(1).unwrap_or_else(|| {
        eprintln!(
            "Usage: {} <path/to/inventory.toml>",
            &env::args().next().expect("args to be > 0")
        );
        process::exit(2);
    })
}
