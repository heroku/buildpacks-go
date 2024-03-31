// Required due to: https://github.com/rust-lang/rust/issues/95513
#![allow(unused_crate_dependencies)]

use heroku_go_utils::vrs::GoVersion;
use heroku_inventory_utils::inv::Inventory;
use heroku_inventory_utils::upstream::UpstreamInventory;

/// Updates the local go inventory.toml with versions published on go.dev.
fn main() {
    Inventory::<GoVersion>::update_local();
}
