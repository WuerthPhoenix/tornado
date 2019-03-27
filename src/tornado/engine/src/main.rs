use crate::config::Command;

pub mod collector;
mod command;
pub mod config;
pub mod dispatcher;
pub mod engine;
pub mod executor;

fn main() -> Result<(), Box<std::error::Error>> {
    let conf = config::Conf::build();
    match &conf.command {
        Command::Check => command::check::check(&conf),
        Command::Daemon { daemon_config } => command::daemon::daemon(&conf, daemon_config.clone()),
    }
}
