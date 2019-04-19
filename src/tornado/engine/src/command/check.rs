use crate::config::{parse_config_files, Conf};
use failure::Fail;
use tornado_engine_matcher::matcher::Matcher;

pub fn check(conf: &Conf) -> Result<(), Box<std::error::Error>> {
    println!("Check Tornado configuration");
    let configs = parse_config_files(conf)?;
    let _matcher = configs
        .matcher_config
        .read()
        .and_then(|config| Matcher::build(&config))
        .map_err(Fail::compat)?;
    println!("The configuration is correct.");
    Ok(())
}
