use typescript_definitions::TypeScriptifyTrait;

use dto::*;
use std::fs::*;
use std::io::Write;
use std::str::FromStr;

const TS_OUTPUT_DIR: &str = "./ts";
const TS_OUTPUT_FILE: &str = "dto.ts";

#[test]
fn generate_ts_files() -> Result<(), Box<std::error::Error>> {
    let custom_types = r#"
/* tslint:disable */

/* ------------ */
/* custom types */
/* ------------ */

export type Value = any;"#;

    let mut ts_code = "".to_owned();

    // Push custom ts types
    push_ts(&mut ts_code, custom_types);

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
    push_ts(&mut ts_code, &config::MatcherConfigDto::type_script_ify());
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

    // println!("{}", ts_code);

    let previous_ts_code = read_ts_file();
    if fail_if_changed()? && !ts_code.eq(&previous_ts_code) {
        assert!(false, "The DTO TS definitions have changed!")
    }
    write_ts_file(&ts_code)
}

fn push_ts(ts_code: &mut String, ts_trait: &str) {
    ts_code.push_str("\n\n");
    ts_code.push_str(ts_trait);
}

fn fail_if_changed() -> Result<bool, Box<std::error::Error>> {
    let fail_if_changed =
        std::env::var("TORNADO_DTO_BUILD_FAIL_IF_CHANGED").unwrap_or("false".to_owned());
    let fail_if_changed = bool::from_str(&fail_if_changed)?;

    println!("Fail if dto changed: {}", fail_if_changed);
    Ok(fail_if_changed)
}

fn read_ts_file() -> String {
    let path = format!("{}/{}", TS_OUTPUT_DIR, TS_OUTPUT_FILE);
    if let Ok(txt) = read_to_string(&path) {
        txt
    } else {
        "".to_owned()
    }
}

fn write_ts_file(ts_code: &str) -> Result<(), Box<std::error::Error>> {
    create_dir_all(TS_OUTPUT_DIR)?;
    let path = format!("{}/{}", TS_OUTPUT_DIR, TS_OUTPUT_FILE);
    let mut ts_file = File::create(path)?;
    Ok(ts_file.write_all(ts_code.as_bytes())?)
}
