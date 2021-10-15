use crate::command::apm_tracing::apm_tracing;
use crate::config::{Opt, SubCommand};
use clap::Clap;

pub mod actor;
mod api;
mod command;
pub mod config;
mod enrich;
mod monitoring;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let opt: Opt = Opt::parse();

    let config_dir = opt.config_dir();
    let rules_dir = opt.rules_dir();
    let drafts_dir = opt.drafts_dir();

    match &opt.command {
        SubCommand::Check => command::check::check(config_dir, rules_dir, drafts_dir).await,
        SubCommand::Daemon => command::daemon::daemon(config_dir, rules_dir, drafts_dir).await,
        SubCommand::RulesUpgrade => {
            command::upgrade_rules::upgrade_rules(config_dir, rules_dir, drafts_dir).await
        }
        SubCommand::FilterCreate(opts) => {
            command::create_filter::create_filter(config_dir, rules_dir, drafts_dir, opts).await
        }
        SubCommand::ApmTracing { command } => apm_tracing(config_dir, command).await,
    }
}
