// Required due to: https://github.com/rust-lang/rust/issues/95513
#![allow(unused_crate_dependencies)]

use heroku_go_utils::inv::list_upstream_artifacts;
use heroku_inventory_utils::inv::Inventory;
use std::{env, fs, process};

/// Updates the local go inventory.toml with versions published on go.dev.
fn main() {
    let filename = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: update_inventory <path/to/inventory.toml>");
        process::exit(2);
    });

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

    fs::write(filename, toml).unwrap_or_else(|e| {
        eprintln!("Error writing inventory to file: {e}");
        process::exit(7);
    });
}
