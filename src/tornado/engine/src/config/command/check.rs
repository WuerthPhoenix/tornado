use tornado_engine_matcher::matcher::Matcher;
use failure::Fail;
use crate::config::{parse_config_files, Conf};

pub fn check(conf: Conf) -> Result<(), Box<std::error::Error>> {
    println!("Check Tornado configuration");
    let configs = parse_config_files(&conf)?;
    let _matcher = Matcher::build(&configs.matcher).map_err(|e| e.compat())?;
    println!("The configuration is correct.");
    Ok(())
}