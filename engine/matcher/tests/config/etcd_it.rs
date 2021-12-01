use tornado_engine_matcher::config::{etcd::{EtcdMatcherConfigManager, editor::{DRAFT_ID, current_ts_ms}}, MatcherConfigReader, MatcherConfigEditor, fs::FsMatcherConfigManager, MatcherConfig, filter::Filter, Defaultable};


    #[tokio::test]
    async fn should_create_a_new_draft_cloning_from_current_config_with_root_filter() {
        // Arrange
        let config_manager = create_config("./test_resources/config_04").await;
        let user_1 = "user_1".to_owned();

        // Act
        let draft_id = config_manager.create_draft(user_1.clone()).await.unwrap();
        let current_config = config_manager.get_config().await.unwrap();
        let current_draft = config_manager.get_draft(&draft_id).await.unwrap();

        // Assert
        assert_eq!(DRAFT_ID, &draft_id);
        assert_eq!(
            current_config,
            current_draft.config
        );

        assert_eq!(DRAFT_ID, current_draft.data.draft_id);
        assert_eq!(user_1, current_draft.data.user);

        // current_config must be a filter for this test
        match current_draft.config {
            MatcherConfig::Filter { .. } => {}
            _ => assert!(false),
        }

    }


    #[tokio::test]
    async fn should_create_a_new_draft_cloning_current_config_with_root_ruleset()  {
        // Arrange
        let config_manager = create_config("./test_resources/rules").await;
        let user_1 = "user_1".to_owned();

        // Act
        let draft_id = config_manager.create_draft(user_1.clone()).await.unwrap();
        let current_config = config_manager.get_config().await.unwrap();
        let current_draft = config_manager.get_draft(&draft_id).await.unwrap();

        // Assert
        assert_eq!(DRAFT_ID, &draft_id);

        // A default root filter should be automatically added
        match current_draft.config {
            MatcherConfig::Filter { name, nodes, .. } => {
                assert_eq!("root", name);
                assert_eq!(1, nodes.len());
                assert_eq!(current_config, nodes[0]);
            }
            _ => assert!(false),
        }

        // current_config must be a ruleset for this test
        match current_config {
            MatcherConfig::Ruleset { .. } => {}
            _ => assert!(false),
        }

    }


    #[tokio::test]
    async fn should_return_a_draft_by_id() {
        // Arrange
        let current_ts_ms = current_ts_ms();
        let config_manager = create_config("./test_resources/config_04").await;
        let user_1 = "user_1".to_owned();

        // Act
        let draft_id = config_manager.create_draft(user_1.clone()).await.unwrap();
        let current_config = config_manager.get_config().await.unwrap();
        let current_draft = config_manager.get_draft(&draft_id).await.unwrap();

        // Assert
        assert_eq!(current_config, current_draft.config);
        assert_eq!(user_1, current_draft.data.user);
        assert!(current_draft.data.created_ts_ms >= current_ts_ms);
        assert_eq!(current_draft.data.updated_ts_ms, current_draft.data.created_ts_ms);

    }


    #[tokio::test]
    async fn get_draft_should_return_error_if_draft_id_does_not_exists() {
        // Arrange
        let config_manager = create_config("./test_resources/config_04").await;

        // Act
        let result = config_manager.get_draft("Hello, World!").await;

        // Assert
        assert!(result.is_err());

    }


    #[tokio::test]
    async fn get_drafts_should_return_all_draft_ids() {
        // Arrange
        let config_manager = create_config("./test_resources/config_04").await;
        let user_1 = "user_1".to_owned();

        // Act
        let drafts_before_create = config_manager.get_drafts().await.unwrap();
        let created_draft_id = config_manager.create_draft(user_1).await.unwrap();
        let drafts_after_create = config_manager.get_drafts().await.unwrap();
        config_manager.delete_draft(&created_draft_id).await.unwrap();
        let drafts_after_delete = config_manager.get_drafts().await.unwrap();

        // Assert
        assert!(drafts_before_create.is_empty());
        assert_eq!(vec![created_draft_id], drafts_after_create);
        assert!(drafts_after_delete.is_empty());

    }

    #[tokio::test]
    async fn should_return_delete_a_draft_by_id() {
        // Arrange
        let config_manager = create_config("./test_resources/config_04").await;
        let user_1 = "user_1".to_owned();

        let created_draft_id = config_manager.create_draft(user_1).await.unwrap();

        // Act
        config_manager.delete_draft(&created_draft_id).await.unwrap();
        let get_attempt_result = config_manager.get_draft(&created_draft_id).await;

        // Assert
        assert!(get_attempt_result.is_err());
        assert!(config_manager.get_drafts().await.unwrap().is_empty());

    }

    #[tokio::test]
    async fn should_update_a_draft_by_id() {
        // Arrange
        let config_manager = create_config("./test_resources/rules").await;

        let new_config =
            FsMatcherConfigManager::new("./test_resources/config_implicit_filter", "")
                .get_config()
                .await
                .unwrap();

        let user_1 = "user_1".to_owned();
        let user_2 = "user_2".to_owned();

        // Act
        let draft_id = config_manager.create_draft(user_1.clone()).await.unwrap();
        let draft_before_update = config_manager.get_draft(&draft_id).await.unwrap();

        std::thread::sleep(std::time::Duration::from_millis(1));

        config_manager.update_draft(&draft_id, user_2.clone(), &new_config).await.unwrap();
        let draft_after_update = config_manager.get_draft(&draft_id).await.unwrap();

        // Assert
        assert_eq!(&user_1, &draft_before_update.data.user);
        assert_eq!(&user_2, &draft_after_update.data.user);
        assert_eq!(draft_after_update.data.created_ts_ms, draft_before_update.data.created_ts_ms);
        assert!(draft_after_update.data.updated_ts_ms > draft_before_update.data.updated_ts_ms);
        assert_ne!(draft_after_update.config, draft_before_update.config);
        assert_eq!(new_config, draft_after_update.config);

    }


    #[tokio::test]
    async fn should_validate_draft_on_update() {
        // Arrange
        let config_manager = create_config("./test_resources/rules").await;

        let config_with_invalid_filter_name = MatcherConfig::Filter {
            name: "filter name with space".to_owned(),
            nodes: vec![],
            filter: Filter {
                filter: Defaultable::Default {},
                active: true,
                description: "".to_owned(),
            },
        };

        let config_with_invalid_rule_name = MatcherConfig::Filter {
            name: "filter".to_owned(),
            nodes: vec![MatcherConfig::Ruleset {
                name: "rule name with space".to_owned(),
                rules: vec![],
            }],
            filter: Filter {
                filter: Defaultable::Default {},
                active: true,
                description: "".to_owned(),
            },
        };

        let user_1 = "user_1".to_owned();
        let draft_id = config_manager.create_draft(user_1.clone()).await.unwrap();

        // Act
        let update_result_1 = config_manager
            .update_draft(&draft_id, user_1.clone(), &config_with_invalid_filter_name)
            .await;
        let update_result_2 = config_manager
            .update_draft(&draft_id, user_1.clone(), &config_with_invalid_rule_name)
            .await;

        // Assert
        assert!(update_result_1.is_err());
        assert!(update_result_2.is_err());
    }

    #[tokio::test]
    async fn should_deploy_a_draft_by_id() {
        // Arrange
        let config_manager = create_config("./test_resources/rules").await;
        let config_before_deploy = config_manager.get_config().await.unwrap();

        let new_config =
            FsMatcherConfigManager::new("./test_resources/config_implicit_filter", "")
                .get_config()
                .await
                .unwrap();

        let user_1 = "user_1".to_owned();
        let user_2 = "user_2".to_owned();

        // Act
        let draft_id = config_manager.create_draft(user_2.clone()).await.unwrap();
        config_manager.update_draft(&draft_id, user_1.clone(), &new_config).await.unwrap();

        // Act
        let deploy_draft_content = config_manager.deploy_draft(&draft_id).await.unwrap();
        let config_after_deploy = config_manager.get_config().await.unwrap();

        // Assert
        assert_ne!(config_before_deploy, config_after_deploy);
        assert_eq!(deploy_draft_content, config_after_deploy);
        assert_eq!(new_config, config_after_deploy);

    }

    #[tokio::test]
    async fn should_take_over_a_draft() {
        // Arrange
        let config_manager = create_config("./test_resources/rules").await;

        let user_1 = "user_1".to_owned();
        let user_2 = "user_2".to_owned();

        let draft_id = config_manager.create_draft(user_1.clone()).await.unwrap();

        // Act
        let draft_before_take_over = config_manager.get_draft(&draft_id).await.unwrap();
        config_manager.draft_take_over(&draft_id, user_2.clone()).await.unwrap();
        let draft_after_take_over = config_manager.get_draft(&draft_id).await.unwrap();

        // Assert
        assert_eq!(user_1, draft_before_take_over.data.user);
        assert_eq!(user_2, draft_after_take_over.data.user);
        assert_eq!(draft_before_take_over.config, draft_after_take_over.config);

    }

    #[tokio::test]
    async fn should_deploy_a_new_config() {
        // Arrange
        let config_manager = create_config("./test_resources/rules").await;
        let config_before_deploy = config_manager.get_config().await.unwrap();

        let new_config =
            FsMatcherConfigManager::new("./test_resources/config_implicit_filter", "")
                .get_config()
                .await
                .unwrap();

        // Act
        let deployed_config = config_manager.deploy_config(&new_config).await.unwrap();

        // Assert
        let config_after_deploy = config_manager.get_config().await.unwrap();
        assert_ne!(config_before_deploy, config_after_deploy);
        assert_eq!(deployed_config, config_after_deploy);
        assert_eq!(new_config, config_after_deploy);

    }


    async fn create_config(rules_source_dir: &str) -> EtcdMatcherConfigManager {
        let base_path = format!("/{}", rand::random::<u32>());
        let etcd_config_manager = EtcdMatcherConfigManager::new(base_path).await.unwrap();
        let matcher_config = FsMatcherConfigManager::new(rules_source_dir, "").get_config().await.unwrap();
        etcd_config_manager.deploy_config(&matcher_config).await.unwrap();
        etcd_config_manager
    }