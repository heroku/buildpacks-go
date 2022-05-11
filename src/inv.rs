use serde::{Deserialize, Serialize};

pub const HOST_BASE_URL: &str = "https://dl.google.com/go";
pub const MIRROR_BASE_URL: &str = "https://heroku-golang-prod.s3.amazonaws.com";
pub const REGION: &str = "us-east-1";
pub const ARCH: &str = "linux-amd64";

/// Represents a collection of known go releases.
#[derive(Debug, Deserialize, Serialize)]
pub struct Inventory {
    pub releases: Vec<Release>,
}

/// Represents a known go release.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Release {
    pub version: String,
    pub sha: String,
    pub name: String,
}

impl Inventory {}
