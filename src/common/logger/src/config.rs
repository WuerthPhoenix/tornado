#[derive(Debug)]
pub struct LoggerConfig {
    pub root_level: String,
    pub output_system_enabled: bool,
    pub output_file_enabled: bool,
    pub output_file_name: String,
}
