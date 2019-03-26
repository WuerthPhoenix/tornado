pub mod collector;
pub mod config;
pub mod dispatcher;
pub mod engine;
pub mod executor;

fn main() -> Result<(), Box<std::error::Error>> {
    let conf = config::Conf::build();
    let command = conf.command.clone();
    command.execute(conf)
}


