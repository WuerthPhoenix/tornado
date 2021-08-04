use serde::{Deserialize, Serialize};
use typescript_definitions::TypeScriptify;

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, TypeScriptify)]
pub struct LoggerConfigDto {
    pub level: String,
    pub stdout_enabled: bool,
    pub apm_enabled: bool,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, TypeScriptify)]
pub struct SetLoggerLevelRequestDto {
    pub level: String,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, TypeScriptify)]
pub struct SetLoggerApmRequestDto {
    pub enabled: bool,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, TypeScriptify)]
pub struct SetLoggerStdoutRequestDto {
    pub enabled: bool,
}