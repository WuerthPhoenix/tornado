use crate::config::parse_config_files;
use tornado_engine_matcher::config::MatcherConfigReader;
use tornado_engine_matcher::matcher::Matcher;

pub async fn check(
    config_dir: &str,
    rules_dir: &str,
    drafts_dir: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    println!("Check Tornado configuration");
    let configs = parse_config_files(config_dir, rules_dir, drafts_dir)?;
    let _matcher =
        configs.matcher_config.get_config().await.and_then(|config| Matcher::build(&config))?;
    println!("The configuration is correct.");
    Ok(())
}
