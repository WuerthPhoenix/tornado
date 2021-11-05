use crate::config::api::{ConfigApi, ConfigApiHandler};
use crate::config::convert::{
    dto_into_matcher_config, matcher_config_draft_into_dto, matcher_config_into_dto,
};
use crate::model::{ApiData, ApiDataV2};
use actix_web::web::{Data, Json, Path};
use actix_web::{web, HttpRequest, Scope};
use log::*;
use serde::Deserialize;
use tornado_engine_api_dto::common::Id;
use tornado_engine_api_dto::config::{
    MatcherConfigDraftDto, MatcherConfigDto, ProcessingTreeNodeConfigDto,
    ProcessingTreeNodeDetailsDto,
};
use tornado_engine_matcher::config::{MatcherConfigEditor, MatcherConfigReader};

pub fn build_config_endpoints<
    A: ConfigApiHandler + 'static,
    CM: MatcherConfigReader + MatcherConfigEditor + 'static,
>(
    data: ApiData<ConfigApi<A, CM>>,
) -> Scope {
    web::scope("/v1_beta/config")
        .app_data(Data::new(data))
        .service(web::resource("/current").route(web::get().to(get_current_configuration::<A, CM>)))
        .service(
            web::resource("/drafts")
                .route(web::get().to(get_drafts::<A, CM>))
                .route(web::post().to(create_draft::<A, CM>)),
        )
        .service(
            web::resource("/drafts/{draft_id}")
                .route(web::get().to(get_draft::<A, CM>))
                .route(web::put().to(update_draft::<A, CM>))
                .route(web::delete().to(delete_draft::<A, CM>)),
        )
        .service(
            web::resource("/drafts/{draft_id}/deploy").route(web::post().to(deploy_draft::<A, CM>)),
        )
        .service(
            web::resource("/drafts/{draft_id}/take_over")
                .route(web::post().to(draft_take_over::<A, CM>)),
        )
}

pub fn build_config_v2_endpoints<
    A: ConfigApiHandler + 'static,
    CM: MatcherConfigReader + MatcherConfigEditor + 'static,
>(
    data: ApiDataV2<ConfigApi<A, CM>>,
) -> Scope {
    web::scope("/config").app_data(Data::new(data)).service(
        web::scope("/active")
            .service(
                web::resource("/tree/children/{auth_key}")
                    .route(web::get().to(get_tree_node::<A, CM>)),
            )
            .service(
                web::resource("/tree/children/{auth_key}/{node_path}")
                    .route(web::get().to(get_tree_node_with_node_path::<A, CM>)),
            )
            .service(
                web::resource("/tree/details/{auth_key}/{node_path}")
                    .route(web::get().to(get_tree_node_details::<A, CM>)),
            ),
    )
}

#[derive(Deserialize)]
struct EndpointPath {
    auth_key: String,
    node_path: String,
}

async fn get_tree_node<
    A: ConfigApiHandler + 'static,
    CM: MatcherConfigReader + MatcherConfigEditor + 'static,
>(
    req: HttpRequest,
    data: Data<ApiDataV2<ConfigApi<A, CM>>>,
    auth_key: Path<String>,
) -> actix_web::Result<Json<Vec<ProcessingTreeNodeConfigDto>>> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req, &auth_key)?;

    let result = data
        .api
        .get_current_config_processing_tree_nodes_by_path(auth_ctx, &"".to_string())
        .await?;
    Ok(Json(result))
}

async fn get_tree_node_with_node_path<
    A: ConfigApiHandler + 'static,
    CM: MatcherConfigReader + MatcherConfigEditor + 'static,
>(
    req: HttpRequest,
    endpoint_params: Path<EndpointPath>,
    data: Data<ApiDataV2<ConfigApi<A, CM>>>,
) -> actix_web::Result<Json<Vec<ProcessingTreeNodeConfigDto>>> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req, &endpoint_params.auth_key)?;
    let result = data
        .api
        .get_current_config_processing_tree_nodes_by_path(auth_ctx, &endpoint_params.node_path)
        .await?;
    Ok(Json(result))
}

async fn get_tree_node_details<
    A: ConfigApiHandler + 'static,
    CM: MatcherConfigReader + MatcherConfigEditor + 'static,
>(
    req: HttpRequest,
    endpoint_params: Path<EndpointPath>,
    data: Data<ApiDataV2<ConfigApi<A, CM>>>,
) -> actix_web::Result<Json<ProcessingTreeNodeDetailsDto>> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req, &endpoint_params.auth_key)?;
    let result = data
        .api
        .get_current_config_node_details_by_path(auth_ctx, &endpoint_params.node_path)
        .await?;
    Ok(Json(result))
}

async fn get_current_configuration<
    A: ConfigApiHandler + 'static,
    CM: MatcherConfigReader + MatcherConfigEditor + 'static,
>(
    req: HttpRequest,
    data: Data<ApiData<ConfigApi<A, CM>>>,
) -> actix_web::Result<Json<MatcherConfigDto>> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req)?;
    let result = data.api.get_current_configuration(auth_ctx).await?;
    let matcher_config_dto = matcher_config_into_dto(result)?;
    Ok(Json(matcher_config_dto))
}

async fn get_drafts<
    A: ConfigApiHandler + 'static,
    CM: MatcherConfigReader + MatcherConfigEditor + 'static,
>(
    req: HttpRequest,
    data: Data<ApiData<ConfigApi<A, CM>>>,
) -> actix_web::Result<Json<Vec<String>>> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req)?;
    let result = data.api.get_drafts(auth_ctx).await?;
    Ok(Json(result))
}

async fn get_draft<
    A: ConfigApiHandler + 'static,
    CM: MatcherConfigReader + MatcherConfigEditor + 'static,
>(
    req: HttpRequest,
    draft_id: Path<String>,
    data: Data<ApiData<ConfigApi<A, CM>>>,
) -> actix_web::Result<Json<MatcherConfigDraftDto>> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req)?;
    let result = data.api.get_draft(auth_ctx, &draft_id.into_inner()).await?;
    let matcher_config_dto = matcher_config_draft_into_dto(result)?;
    Ok(Json(matcher_config_dto))
}

async fn create_draft<
    A: ConfigApiHandler + 'static,
    CM: MatcherConfigReader + MatcherConfigEditor + 'static,
>(
    req: HttpRequest,
    data: Data<ApiData<ConfigApi<A, CM>>>,
) -> actix_web::Result<Json<Id<String>>> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req)?;
    let result = data.api.create_draft(auth_ctx).await?;
    Ok(Json(result))
}

async fn update_draft<
    A: ConfigApiHandler + 'static,
    CM: MatcherConfigReader + MatcherConfigEditor + 'static,
>(
    req: HttpRequest,
    draft_id: Path<String>,
    body: Json<MatcherConfigDto>,
    data: Data<ApiData<ConfigApi<A, CM>>>,
) -> actix_web::Result<Json<()>> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req)?;
    let config = dto_into_matcher_config(body.into_inner())?;
    data.api.update_draft(auth_ctx, &draft_id.into_inner(), config).await?;
    Ok(Json(()))
}

async fn delete_draft<
    A: ConfigApiHandler + 'static,
    CM: MatcherConfigReader + MatcherConfigEditor + 'static,
>(
    req: HttpRequest,
    draft_id: Path<String>,
    data: Data<ApiData<ConfigApi<A, CM>>>,
) -> actix_web::Result<Json<()>> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req)?;
    data.api.delete_draft(auth_ctx, &draft_id.into_inner()).await?;
    Ok(Json(()))
}

async fn deploy_draft<
    A: ConfigApiHandler + 'static,
    CM: MatcherConfigReader + MatcherConfigEditor + 'static,
>(
    req: HttpRequest,
    draft_id: Path<String>,
    data: Data<ApiData<ConfigApi<A, CM>>>,
) -> actix_web::Result<Json<MatcherConfigDto>> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req)?;
    let result = data.api.deploy_draft(auth_ctx, &draft_id.into_inner()).await?;
    let matcher_config_dto = matcher_config_into_dto(result)?;
    Ok(Json(matcher_config_dto))
}

async fn draft_take_over<
    A: ConfigApiHandler + 'static,
    CM: MatcherConfigReader + MatcherConfigEditor + 'static,
>(
    req: HttpRequest,
    draft_id: Path<String>,
    data: Data<ApiData<ConfigApi<A, CM>>>,
) -> actix_web::Result<Json<()>> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req)?;
    data.api.draft_take_over(auth_ctx, &draft_id.into_inner()).await?;
    Ok(Json(()))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::auth::auth_v2::AuthServiceV2;
    use crate::auth::test::{test_auth_service, test_auth_service_v2};
    use crate::auth::AuthService;
    use crate::error::ApiError;
    use actix_web::{
        http::{header, StatusCode},
        test, App,
    };
    use async_trait::async_trait;
    use maplit::hashmap;
    use std::sync::Arc;
    use tornado_engine_api_dto::auth::Auth;
    use tornado_engine_api_dto::auth_v2::{AuthHeaderV2, Authorization};
    use tornado_engine_api_dto::config::FilterDto;
    use tornado_engine_matcher::config::filter::Filter;
    use tornado_engine_matcher::config::{
        Defaultable, MatcherConfig, MatcherConfigDraft, MatcherConfigDraftData,
    };
    use tornado_engine_matcher::error::MatcherError;

    struct ConfigManager {}

    #[async_trait::async_trait(?Send)]
    impl MatcherConfigReader for ConfigManager {
        async fn get_config(&self) -> Result<MatcherConfig, MatcherError> {
            Ok(MatcherConfig::Filter {
                name: "root".to_owned(),
                filter: Filter {
                    description: "".to_string(),
                    filter: Defaultable::Default {},
                    active: false,
                },
                nodes: vec![],
            })
        }
    }

    #[async_trait::async_trait(?Send)]
    impl MatcherConfigEditor for ConfigManager {
        async fn get_drafts(&self) -> Result<Vec<String>, MatcherError> {
            unimplemented!()
        }

        async fn get_draft(&self, draft_id: &str) -> Result<MatcherConfigDraft, MatcherError> {
            Ok(MatcherConfigDraft {
                data: MatcherConfigDraftData {
                    user: "user".to_owned(),
                    draft_id: draft_id.to_owned(),
                    created_ts_ms: 0,
                    updated_ts_ms: 0,
                },
                config: MatcherConfig::Ruleset { name: "ruleset".to_owned(), rules: vec![] },
            })
        }

        async fn create_draft(&self, _user: String) -> Result<String, MatcherError> {
            unimplemented!()
        }

        async fn update_draft(
            &self,
            _draft_id: &str,
            _user: String,
            _config: &MatcherConfig,
        ) -> Result<(), MatcherError> {
            unimplemented!()
        }

        async fn deploy_draft(&self, _draft_id: &str) -> Result<MatcherConfig, MatcherError> {
            Ok(MatcherConfig::Ruleset { name: "ruleset_new".to_owned(), rules: vec![] })
        }

        async fn delete_draft(&self, _draft_id: &str) -> Result<(), MatcherError> {
            unimplemented!()
        }

        async fn draft_take_over(
            &self,
            _draft_id: &str,
            _user: String,
        ) -> Result<(), MatcherError> {
            Ok(())
        }

        async fn deploy_config(
            &self,
            _config: &MatcherConfig,
        ) -> Result<MatcherConfig, MatcherError> {
            unimplemented!()
        }
    }

    struct TestApiHandler {}

    #[async_trait(?Send)]
    impl ConfigApiHandler for TestApiHandler {
        async fn reload_configuration(&self) -> Result<MatcherConfig, ApiError> {
            Ok(MatcherConfig::Ruleset { name: "ruleset_new".to_owned(), rules: vec![] })
        }
    }

    #[actix_rt::test]
    async fn current_config_should_return_status_code_unauthorized_if_no_token(
    ) -> Result<(), ApiError> {
        // Arrange
        let mut srv = test::init_service(App::new().service(build_config_endpoints(ApiData {
            auth: test_auth_service(),
            api: ConfigApi::new(TestApiHandler {}, Arc::new(ConfigManager {})),
        })))
        .await;

        // Act
        let request = test::TestRequest::get().uri("/v1_beta/config/current").to_request();

        let response = test::call_service(&mut srv, request).await;

        // Assert
        assert_eq!(StatusCode::UNAUTHORIZED, response.status());
        Ok(())
    }

    #[actix_rt::test]
    async fn current_config_should_return_status_code_unauthorized_if_no_view_permission(
    ) -> Result<(), ApiError> {
        // Arrange
        let mut srv = test::init_service(App::new().service(build_config_endpoints(ApiData {
            auth: test_auth_service(),
            api: ConfigApi::new(TestApiHandler {}, Arc::new(ConfigManager {})),
        })))
        .await;

        // Act
        let request = test::TestRequest::get()
            .insert_header((
                header::AUTHORIZATION,
                AuthService::auth_to_token_header(&Auth::new("user", vec![""]))?,
            ))
            .uri("/v1_beta/config/current")
            .to_request();

        let response = test::call_service(&mut srv, request).await;

        // Assert
        assert_eq!(StatusCode::FORBIDDEN, response.status());
        Ok(())
    }

    #[actix_rt::test]
    async fn should_return_status_code_ok() -> Result<(), ApiError> {
        // Arrange
        let mut srv = test::init_service(App::new().service(build_config_endpoints(ApiData {
            auth: test_auth_service(),
            api: ConfigApi::new(TestApiHandler {}, Arc::new(ConfigManager {})),
        })))
        .await;

        // Act
        let request = test::TestRequest::get()
            .insert_header((
                header::AUTHORIZATION,
                AuthService::auth_to_token_header(&Auth::new("user", vec!["edit"]))?,
            ))
            .uri("/v1_beta/config/current")
            .to_request();

        let response = test::call_service(&mut srv, request).await;

        // Assert
        assert_eq!(StatusCode::OK, response.status());
        Ok(())
    }

    #[actix_rt::test]
    async fn should_return_the_matcher_config() -> Result<(), ApiError> {
        // Arrange
        let mut srv = test::init_service(App::new().service(build_config_endpoints(ApiData {
            auth: test_auth_service(),
            api: ConfigApi::new(TestApiHandler {}, Arc::new(ConfigManager {})),
        })))
        .await;

        // Act
        let request = test::TestRequest::get()
            .insert_header((
                header::AUTHORIZATION,
                AuthService::auth_to_token_header(&Auth::new("user", vec!["edit"]))?,
            ))
            .uri("/v1_beta/config/current")
            .to_request();

        // Assert
        let dto: tornado_engine_api_dto::config::MatcherConfigDto =
            test::read_response_json(&mut srv, request).await;

        assert_eq!(
            tornado_engine_api_dto::config::MatcherConfigDto::Filter {
                name: "root".to_owned(),
                filter: FilterDto { description: "".to_string(), active: false, filter: None },
                nodes: vec![]
            },
            dto
        );

        Ok(())
    }

    #[actix_rt::test]
    async fn should_return_the_reloaded_matcher_config() -> Result<(), ApiError> {
        // Arrange
        let mut srv = test::init_service(App::new().service(build_config_endpoints(ApiData {
            auth: test_auth_service(),
            api: ConfigApi::new(TestApiHandler {}, Arc::new(ConfigManager {})),
        })))
        .await;

        // Act
        let request = test::TestRequest::post()
            .insert_header((
                header::AUTHORIZATION,
                AuthService::auth_to_token_header(&Auth::new("user", vec!["edit"]))?,
            ))
            .uri("/v1_beta/config/drafts/1/deploy")
            .to_request();

        // Assert
        let dto: tornado_engine_api_dto::config::MatcherConfigDto =
            test::read_response_json(&mut srv, request).await;

        assert_eq!(
            tornado_engine_api_dto::config::MatcherConfigDto::Ruleset {
                name: "ruleset_new".to_owned(),
                rules: vec![]
            },
            dto
        );

        Ok(())
    }

    #[actix_rt::test]
    async fn should_have_a_draft_take_over_post_endpoint() -> Result<(), ApiError> {
        // Arrange
        let mut srv = test::init_service(App::new().service(build_config_endpoints(ApiData {
            auth: test_auth_service(),
            api: ConfigApi::new(TestApiHandler {}, Arc::new(ConfigManager {})),
        })))
        .await;

        // Act
        let request = test::TestRequest::post()
            .insert_header((
                header::AUTHORIZATION,
                AuthService::auth_to_token_header(&Auth::new("user", vec!["edit"]))?,
            ))
            .uri("/v1_beta/config/drafts/draft123/take_over")
            .to_request();

        let response = test::call_service(&mut srv, request).await;

        // Assert
        assert_eq!(StatusCode::OK, response.status());
        Ok(())
    }

    #[actix_rt::test]
    async fn v2_endpoint_get_children_should_return_status_code_ok() -> Result<(), ApiError> {
        // Arrange
        let mut srv =
            test::init_service(App::new().service(build_config_v2_endpoints(ApiDataV2 {
                auth: test_auth_service_v2(),
                api: ConfigApi::new(TestApiHandler {}, Arc::new(ConfigManager {})),
            })))
            .await;

        // Act
        let request = test::TestRequest::get()
            .insert_header((
                header::AUTHORIZATION,
                AuthServiceV2::auth_to_token_header(&AuthHeaderV2 {
                    user: "admin".to_string(),
                    auths: hashmap! {
                        "auth1".to_owned() => Authorization {
                            path: vec!["root".to_owned()],
                            roles: vec!["view".to_owned()],
                        }
                    },
                    preferences: None,
                })?,
            ))
            .uri("/config/active/tree/children/auth1")
            .to_request();

        let response = test::call_service(&mut srv, request).await;

        // Assert
        assert_eq!(StatusCode::OK, response.status());
        Ok(())
    }

    #[actix_rt::test]
    async fn v2_endpoint_get_children_by_node_path_should_return_status_code_ok(
    ) -> Result<(), ApiError> {
        // Arrange
        let mut srv =
            test::init_service(App::new().service(build_config_v2_endpoints(ApiDataV2 {
                auth: test_auth_service_v2(),
                api: ConfigApi::new(TestApiHandler {}, Arc::new(ConfigManager {})),
            })))
            .await;

        // Act
        let request = test::TestRequest::get()
            .insert_header((
                header::AUTHORIZATION,
                AuthServiceV2::auth_to_token_header(&AuthHeaderV2 {
                    user: "admin".to_string(),
                    auths: hashmap! {
                        "auth1".to_owned() => Authorization {
                            path: vec!["root".to_owned()],
                            roles: vec!["view".to_owned(), "edit".to_owned()],
                        }
                    },
                    preferences: None,
                })?,
            ))
            .uri("/config/active/tree/children/auth1/root")
            .to_request();

        let response = test::call_service(&mut srv, request).await;

        println!("{:?}", response);
        // Assert
        assert_eq!(StatusCode::OK, response.status());
        Ok(())
    }

    #[actix_rt::test]
    async fn v2_endpoint_get_details_by_node_path_should_return_status_code_ok(
    ) -> Result<(), ApiError> {
        // Arrange
        let mut srv =
            test::init_service(App::new().service(build_config_v2_endpoints(ApiDataV2 {
                auth: test_auth_service_v2(),
                api: ConfigApi::new(TestApiHandler {}, Arc::new(ConfigManager {})),
            })))
            .await;

        // Act
        let request = test::TestRequest::get()
            .insert_header((
                header::AUTHORIZATION,
                AuthServiceV2::auth_to_token_header(&AuthHeaderV2 {
                    user: "admin".to_string(),
                    auths: hashmap! {
                        "auth1".to_owned() => Authorization {
                            path: vec!["root".to_owned()],
                            roles: vec!["view".to_owned()],
                        }
                    },
                    preferences: None,
                })?,
            ))
            .uri("/config/active/tree/details/auth1/root")
            .to_request();

        let response = test::call_service(&mut srv, request).await;

        // Assert
        assert_eq!(StatusCode::OK, response.status());
        Ok(())
    }
}
