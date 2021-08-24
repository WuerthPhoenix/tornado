use serde::{Deserialize, Serialize};
use typescript_definitions::TypeScriptify;
use ajars::Rest;

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

pub const SET_APM_FIRST_CONFIG_REST: Rest<SetApmFirstConfigurationRequestDto, ()> = Rest::post("/set_apm_first_configuration");

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, TypeScriptify)]
pub struct SetApmFirstConfigurationRequestDto {
    pub logger_level: Option<String>,
}

pub const SET_STDOUT_FIRST_CONFIG_REST: Rest<SetStdoutFirstConfigurationRequestDto, ()> = Rest::post("/set_stdout_first_configuration");

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, TypeScriptify)]
pub struct SetStdoutFirstConfigurationRequestDto {}