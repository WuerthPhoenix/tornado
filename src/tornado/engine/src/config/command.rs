use structopt::StructOpt;

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
                println!("Execute config check");
                Ok(())
            }
        }
    }
}
