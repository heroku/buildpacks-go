use serde::{de::DeserializeOwned, Serialize};

pub trait Version: Serialize + DeserializeOwned {}

pub trait VersionRequirement<V> {
    fn satisfies(&self, version: &V) -> bool;
}
