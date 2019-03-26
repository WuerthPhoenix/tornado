use structopt::StructOpt;
use failure::Fail;

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
                let _config_rules = super::load_rules(&conf).map_err(|e| e.compat())?;
                let _icinga2_client_config = super::build_icinga2_client_config(&conf)?;
                let _archive_config = super::build_archive_config(&conf)?;
                println!("The configuration is correct.");
                Ok(())
            }
        }
    }
}
