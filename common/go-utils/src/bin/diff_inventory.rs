// Required due to: https://github.com/rust-lang/rust/issues/95513
#![allow(unused_crate_dependencies)]

use heroku_go_utils::vrs::GoVersion;
use heroku_inventory_utils::inv::Inventory;
use heroku_inventory_utils::upstream::UpstreamInventory;

/// Prints a human-readable software inventory difference. Useful
/// for generating commit messages and changelogs for automated inventory
/// updates.
fn main() {
    Inventory::<GoVersion>::diff_inventory();
}
