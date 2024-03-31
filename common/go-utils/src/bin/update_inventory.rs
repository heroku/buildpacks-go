// Required due to: https://github.com/rust-lang/rust/issues/95513
#![allow(unused_crate_dependencies)]

use heroku_go_utils::vrs::GoVersion;
use heroku_inventory_utils::inv::Inventory;
use heroku_inventory_utils::upstream::UpstreamInventory;
use std::{env, process};

/// Updates the local go inventory.toml with versions published on go.dev.
fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: update_inventory <path/to/inventory.toml>");
        process::exit(2);
    });

    Inventory::<GoVersion>::update_local(path);
}
