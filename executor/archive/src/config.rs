use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize, Clone)]
pub struct ArchiveConfig {
    /// The base path where the archive executor writes into.
    /// All other paths are relative to this one.
    pub base_path: String,

    /// The relative default path
    pub default_path: String,

    /// A map of relative path groups
    pub paths: HashMap<String, String>,

    /// The file cache size for storing open files
    pub file_cache_size: usize,

    /// The file cache Time-To-Live in seconds
    pub file_cache_ttl_secs: u64,
}
