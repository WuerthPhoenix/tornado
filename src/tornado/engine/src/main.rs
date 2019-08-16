use crate::config::Command;

mod api;
mod command;
pub mod config;
pub mod dispatcher;
pub mod engine;
pub mod executor;
mod monitoring;

fn main() -> Result<(), Box<std::error::Error>> {
    let conf = config::Conf::build();
    match &conf.command {
        Command::Check => command::check::check(&conf),
        Command::Daemon => command::daemon::daemon(&conf),
    }
}
