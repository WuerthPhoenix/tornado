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

pub const SET_APM_PRIORITY_CONFIG_REST: Rest<SetApmPriorityConfigurationRequestDto, ()> = Rest::post("/logger/set_apm_priority_configuration");

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, TypeScriptify)]
pub struct SetApmPriorityConfigurationRequestDto {
    pub logger_level: Option<String>,
}

pub const SET_STDOUT_PRIORITY_CONFIG_REST: Rest<SetStdoutPriorityConfigurationRequestDto, ()> = Rest::post("/logger/set_stdout_priority_configuration");

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, TypeScriptify)]
pub struct SetStdoutPriorityConfigurationRequestDto {}