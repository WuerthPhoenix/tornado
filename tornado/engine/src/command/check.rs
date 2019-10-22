use crate::config::parse_config_files;
use failure::Fail;
use tornado_engine_matcher::matcher::Matcher;

pub fn check(config_dir: &str, rules_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Check Tornado configuration");
    let configs = parse_config_files(config_dir, rules_dir)?;
    let _matcher = configs
        .matcher_config
        .read()
        .and_then(|config| Matcher::build(&config))
        .map_err(Fail::compat)?;
    println!("The configuration is correct.");
    Ok(())
}
