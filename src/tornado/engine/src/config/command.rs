use structopt::StructOpt;

mod check;
mod daemon;

impl Command {
    pub fn execute(self, conf: super::Conf) -> Result<(), Box<std::error::Error>> {
        match self {
            Command::Check => check::check(conf),
            Command::Daemon { daemon_config } => daemon::daemon(conf, daemon_config),
        }
    }
}

#[derive(StructOpt, Debug, Clone)]
pub enum Command {
    #[structopt(name = "check")]
    /// Checks that the configuration is valid.
    Check,
    #[structopt(name = "daemon")]
    /// Starts the Tornado daemon
    Daemon {
        #[structopt(flatten)]
        daemon_config: DaemonCommandConfig,
    },
}

#[derive(Debug, StructOpt, Clone)]
#[structopt(rename_all = "kebab-case")]
pub struct DaemonCommandConfig {
    /// The IP address where we will listen for incoming events.
    #[structopt(long, default_value = "127.0.0.1")]
    pub event_socket_ip: String,

    /// The port where we will listen for incoming events.
    #[structopt(long, default_value = "4747")]
    pub event_socket_port: u16,

    /// The IP address where we will listen for incoming snmptrapd events.
    #[structopt(long, default_value = "127.0.0.1")]
    pub snmptrapd_socket_ip: String,

    /// The port where we will listen for incoming snmptrapd events.
    #[structopt(long, default_value = "4748")]
    pub snmptrapd_socket_port: u16,
}
