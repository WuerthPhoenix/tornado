use tornado_engine_matcher::config::{etcd::EtcdMatcherConfigManager, fs::FsMatcherConfigManager, MatcherConfigReader, MatcherConfigEditor};

// Not a real test, it opies the tornado config from FS to ETCD
#[tokio::test]
async fn should_create_etcd_config() {
    let matcher_config = FsMatcherConfigManager::new("./config/rules.d", "").get_config().await.unwrap();
    let etcd_config_manager = EtcdMatcherConfigManager::new("".to_owned()).await.unwrap();
    etcd_config_manager.deploy_config(&matcher_config).await.unwrap();
}