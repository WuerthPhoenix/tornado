use crate::config::{SUBCOMMAND_CHECK, SUBCOMMAND_DAEMON, SUBCOMMAND_RULES_UPGRADE};
use log::error;

pub mod actor;
mod api;
mod command;
pub mod config;
mod monitoring;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let arg_matches = config::arg_matches();

    let config_dir = arg_matches.value_of("config-dir").expect("config-dir should be provided");
    let rules_dir = arg_matches.value_of("rules-dir").expect("rules-dir should be provided");
    let drafts_dir = arg_matches.value_of("drafts-dir").expect("drafts-dir should be provided");

    let subcommand = arg_matches.subcommand();

    match subcommand {
        (SUBCOMMAND_CHECK, _) => command::check::check(config_dir, rules_dir, drafts_dir),
        (SUBCOMMAND_DAEMON, _) => command::daemon::daemon(config_dir, rules_dir, drafts_dir).await,
        (SUBCOMMAND_RULES_UPGRADE, _) => {
            command::upgrade_rules::upgrade_rules(config_dir, rules_dir, drafts_dir)
        }
        _ => {
            error!("Unknown subcommand [{}]", subcommand.0);
            Ok(())
        }
    }
}
