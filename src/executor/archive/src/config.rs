use std::collections::HashMap;

pub struct ArchiveConfig {
    /// The base path where the archive executor writes into.
    /// All other paths are relative to this one.
    pub base_path: String,

    /// The relative default path
    pub default_path: String,

    /// A map of relative path groups.
    pub paths: HashMap<String, String>,
}
