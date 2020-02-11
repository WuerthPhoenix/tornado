use log::error;

mod api;
mod command;
pub mod config;
pub mod dispatcher;
pub mod engine;
pub mod executor;
mod monitoring;

#[actix_rt::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let arg_matches = config::arg_matches();

    let config_dir = arg_matches.value_of("config-dir").expect("config-dir should be provided");
    let rules_dir = arg_matches.value_of("rules-dir").expect("rules-dir should be provided");

    let subcommand = arg_matches.subcommand();
    match subcommand {
        ("check", _) => command::check::check(config_dir, rules_dir),
        ("daemon", _) => command::daemon::daemon(config_dir, rules_dir).await,
        _ => {
            error!("Unknown subcommand [{}]", subcommand.0);
            Ok(())
        }
    }
}
