use structopt::StructOpt;

mod check;
mod daemon;

#[derive(StructOpt, Debug, Clone)]
pub enum Command {
    #[structopt(name = "check")]
    /// Checks that the configuration is valid.
    Check,
    #[structopt(name = "daemon")]
    /// Starts the Tornado daemon
    Daemon
}

impl Command {
    pub fn execute(&self, conf: super::Conf) -> Result<(), Box<std::error::Error>> {
        match self {
            Command::Check => check::check(conf),
            Command::Daemon => daemon::daemon(conf),
        }
    }
}
