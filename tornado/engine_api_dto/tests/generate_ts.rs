// It works only in debug mode (i.e. not in release mode)
#![cfg(debug_assertions)]

use typescript_definitions::TypeScriptifyTrait;

use std::fs::*;
use std::io::Write;
use std::str::FromStr;
use tornado_engine_api_dto::*;
use tornado_engine_matcher as matcher;

const TS_OUTPUT_DIR: &str = "./ts";
const TS_OUTPUT_FILE: &str = "dto.ts";
const TORNADO_DTO_BUILD_REGENERATE_TS_FILES: &str = "TORNADO_DTO_BUILD_REGENERATE_TS_FILES";

///
/// This test verifies that the rust structs and the TS definitions in the generated *.ts files
/// are coherent.
/// To regenerate the *.ts files, launch this test setting the environment variable
/// TORNADO_DTO_BUILD_REGENERATE_TS_FILES to true.
///
/// E.g.:
/// TORNADO_DTO_BUILD_REGENERATE_TS_FILES=true cargo test
///
#[test]
fn check_ts_file_status() -> Result<(), Box<dyn std::error::Error>> {
    let ts_definitions = generate_ts_definitions();
    // println!("{}", ts_code);

    let regenerate_ts_file = is_regenerate_ts_file()?;

    if regenerate_ts_file {
        write_ts_file(&ts_definitions)?;
    }

    let previous_ts_definitions = read_ts_file();
    assert_eq!(previous_ts_definitions, ts_definitions, "\n\nError: The DTO TS definitions are changed but the *.ts files have not being updated!\n\n");
    Ok(())
}

// Whether the ts files need to be regenerated
fn is_regenerate_ts_file() -> Result<bool, Box<dyn std::error::Error>> {
    let fail_if_changed =
        std::env::var(TORNADO_DTO_BUILD_REGENERATE_TS_FILES).unwrap_or("false".to_owned());
    let regenerate_files = bool::from_str(&fail_if_changed)?;

    println!("Should DTO TS files be regenerated? {}", regenerate_files);
    Ok(regenerate_files)
}

// Reads the content of the current ts file from the file system
fn read_ts_file() -> String {
    let path = format!("{}/{}", TS_OUTPUT_DIR, TS_OUTPUT_FILE);
    if let Ok(txt) = read_to_string(&path) {
        txt
    } else {
        "".to_owned()
    }
}

// Writes the ts definitions to the file system
fn write_ts_file(ts_code: &str) -> Result<(), Box<dyn std::error::Error>> {
    create_dir_all(TS_OUTPUT_DIR)?;
    let path = format!("{}/{}", TS_OUTPUT_DIR, TS_OUTPUT_FILE);
    let mut ts_file = File::create(path)?;
    Ok(ts_file.write_all(ts_code.as_bytes())?)
}

// Generates the TS definitions from the source rust code
fn generate_ts_definitions() -> String {
    let custom_types = r#"
/* tslint:disable */

/* WARNING: this file was automatically generated at compile time */
/* DO NOT CHANGE IT MANUALLY */

/* ------------ */
/* custom types */
/* ------------ */

export type Value = any;"#;

    let mut ts_code = "".to_owned();

    // Push custom ts types
    push_ts(&mut ts_code, custom_types);

    // Push 'common' ts types
    push_ts(
        &mut ts_code,
        r#"
/* ---------------- */
/* 'common' types   */
/* ---------------- */"#,
    );
    push_ts(&mut ts_code, &common::Id::<()>::type_script_ify());
    push_ts(&mut ts_code, &common::WebError::type_script_ify());

    // Push 'auth' ts types
    push_ts(
        &mut ts_code,
        r#"
/* -------------- */
/* 'auth' types   */
/* -------------- */"#,
    );
    push_ts(&mut ts_code, &auth::Auth::type_script_ify());
    push_ts(&mut ts_code, &auth::AuthWithPermissionsDto::type_script_ify());
    push_ts(&mut ts_code, &auth::PermissionDto::type_script_ify());
    push_ts(&mut ts_code, &auth::UserPreferences::type_script_ify());

    // Push 'config' ts types
    push_ts(
        &mut ts_code,
        r#"
/* -------------- */
/* 'config' types */
/* -------------- */"#,
    );
    push_ts(&mut ts_code, &config::ActionDto::type_script_ify());
    push_ts(&mut ts_code, &config::ConstraintDto::type_script_ify());
    push_ts(&mut ts_code, &config::ExtractorDto::type_script_ify());
    push_ts(&mut ts_code, &config::ExtractorRegexDto::type_script_ify());
    push_ts(&mut ts_code, &config::FilterDto::type_script_ify());
    push_ts(&mut ts_code, &config::MatcherConfigDraftDataDto::type_script_ify());
    push_ts(&mut ts_code, &config::MatcherConfigDraftDto::type_script_ify());
    push_ts(&mut ts_code, &config::MatcherConfigDto::type_script_ify());
    push_ts(&mut ts_code, &config::ModifierDto::type_script_ify());
    push_ts(&mut ts_code, &config::OperatorDto::type_script_ify());
    push_ts(&mut ts_code, &config::RuleDto::type_script_ify());

    // Push 'event' ts types
    push_ts(
        &mut ts_code,
        r#"
/* ------------- */
/* 'event' types */
/* ------------- */"#,
    );
    push_ts(&mut ts_code, &event::EventDto::type_script_ify());
    push_ts(&mut ts_code, &event::ProcessType::type_script_ify());
    push_ts(&mut ts_code, &event::ProcessedEventDto::type_script_ify());
    push_ts(&mut ts_code, &event::ProcessedFilterDto::type_script_ify());
    push_ts(&mut ts_code, &event::ProcessedFilterStatusDto::type_script_ify());
    push_ts(&mut ts_code, &event::ProcessedNodeDto::type_script_ify());
    push_ts(&mut ts_code, &event::ProcessedRuleDto::type_script_ify());
    push_ts(&mut ts_code, &event::ProcessedRulesDto::type_script_ify());
    push_ts(&mut ts_code, &event::ProcessedRuleStatusDto::type_script_ify());
    push_ts(&mut ts_code, &event::SendEventRequestDto::type_script_ify());

    // Push 'matcher' ts types
    push_ts(
        &mut ts_code,
        r#"
/* ---------------- */
/* 'matcher' types   */
/* ---------------- */"#,
    );
    push_ts(&mut ts_code, &matcher::model::ActionMetaData::type_script_ify());
    push_ts(&mut ts_code, &matcher::model::EnrichedValue::type_script_ify());
    push_ts(&mut ts_code, &matcher::model::EnrichedValueContent::type_script_ify());
    push_ts(&mut ts_code, &matcher::model::ProcessedRuleMetaData::type_script_ify());
    push_ts(&mut ts_code, &matcher::model::ValueMetaData::type_script_ify());

    ts_code
}

fn push_ts(ts_code: &mut String, ts_trait: &str) {
    ts_code.push_str("\n\n");
    ts_code.push_str(ts_trait);
}
