use crate::config::{parse_config_files, Conf};
use failure::Fail;
use tornado_engine_matcher::matcher::Matcher;

pub fn check(conf: &Conf) -> Result<(), Box<std::error::Error>> {
    println!("Check Tornado configuration");
    let configs = parse_config_files(conf)?;
    let _matcher = Matcher::build(&configs.matcher).map_err(Fail::compat)?;
    println!("The configuration is correct.");
    Ok(())
}
