use failure::Fail;
use structopt::StructOpt;
use tornado_engine_matcher::matcher::Matcher;

#[derive(StructOpt, Debug)]
pub enum Command {
    #[structopt(name = "check")]
    /// Checks that the configuration is valid.
    Check,
}

impl Command {
    pub fn execute(&self, conf: &super::Conf) -> Result<(), Box<std::error::Error>> {
        match self {
            Command::Check => {
                println!("Check Tornado configuration");
                let configs = super::parse_config_files(&conf)?;
                let _matcher = Matcher::build(&configs.matcher).map_err(|e| e.compat())?;
                println!("The configuration is correct.");
                Ok(())
            }
        }
    }
}
