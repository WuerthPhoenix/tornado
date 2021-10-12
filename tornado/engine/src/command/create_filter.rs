use crate::config::FilterCreateOpt;

pub async fn create_filter(
    _config_dir: &str,
    _rules_dir: &str,
    _drafts_dir: &str,
    opts: &FilterCreateOpt
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    println!("Creating Filter with name {}. Filter definition will be read from {}", opts.name, opts.from_filepath);
    Ok(())
}
