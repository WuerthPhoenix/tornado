use crate::config::api::{ConfigApi, ConfigApiHandler};
use crate::config::convert::{dto_into_matcher_config, matcher_config_into_dto};
use crate::model::ApiData;
use actix_web::web::{Data, Json, Path};
use actix_web::{web, HttpRequest, Scope};
use log::*;
use tornado_engine_api_dto::common::Id;
use tornado_engine_api_dto::config::MatcherConfigDto;

pub fn build_config_endpoints<C: ConfigApiHandler + 'static>(data: ApiData<ConfigApi<C>>) -> Scope {
    web::scope("/v1/config")
        .data(data)
        .service(web::resource("/current").route(web::get().to(get_current_configuration::<C>)))
        .service(web::resource("/deploy/{draft_id}").route(web::post().to(deploy_draft::<C>)))
        .service(web::resource("/drafts").route(web::get().to(get_drafts::<C>)))
        .service(web::resource("/draft").route(web::post().to(create_draft::<C>)))
        .service(
            web::resource("/draft/{draft_id}")
                .route(web::get().to(get_draft::<C>))
                .route(web::put().to(update_draft::<C>))
                .route(web::delete().to(delete_draft::<C>)),
        )
}

async fn get_current_configuration<C: ConfigApiHandler + 'static>(
    req: HttpRequest,
    data: Data<ApiData<ConfigApi<C>>>,
) -> actix_web::Result<Json<MatcherConfigDto>> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req)?;
    let result = data.api.get_current_configuration(auth_ctx).await?;
    let matcher_config_dto = matcher_config_into_dto(result)?;
    Ok(Json(matcher_config_dto))
}

async fn get_drafts<C: ConfigApiHandler + 'static>(
    req: HttpRequest,
    data: Data<ApiData<ConfigApi<C>>>,
) -> actix_web::Result<Json<Vec<String>>> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req)?;
    let result = data.api.get_drafts(auth_ctx).await?;
    Ok(Json(result))
}

async fn get_draft<C: ConfigApiHandler + 'static>(
    req: HttpRequest,
    draft_id: Path<String>,
    data: Data<ApiData<ConfigApi<C>>>,
) -> actix_web::Result<Json<MatcherConfigDto>> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req)?;
    let result = data.api.get_draft(auth_ctx, draft_id.into_inner()).await?;
    let matcher_config_dto = matcher_config_into_dto(result)?;
    Ok(Json(matcher_config_dto))
}

async fn create_draft<C: ConfigApiHandler + 'static>(
    req: HttpRequest,
    data: Data<ApiData<ConfigApi<C>>>,
) -> actix_web::Result<Json<Id<String>>> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req)?;
    let result = data.api.create_draft(auth_ctx).await?;
    Ok(Json(result))
}

async fn update_draft<C: ConfigApiHandler + 'static>(
    req: HttpRequest,
    draft_id: Path<String>,
    body: Json<MatcherConfigDto>,
    data: Data<ApiData<ConfigApi<C>>>,
) -> actix_web::Result<Json<()>> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req)?;
    let config = dto_into_matcher_config(body.into_inner())?;
    data.api.update_draft(auth_ctx, draft_id.into_inner(), config).await?;
    Ok(Json(()))
}

async fn delete_draft<C: ConfigApiHandler + 'static>(
    req: HttpRequest,
    draft_id: Path<String>,
    data: Data<ApiData<ConfigApi<C>>>,
) -> actix_web::Result<Json<()>> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req)?;
    data.api.delete_draft(auth_ctx, draft_id.into_inner()).await?;
    Ok(Json(()))
}

async fn deploy_draft<C: ConfigApiHandler + 'static>(
    req: HttpRequest,
    draft_id: Path<String>,
    data: Data<ApiData<ConfigApi<C>>>,
) -> actix_web::Result<Json<MatcherConfigDto>> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req)?;
    let result = data.api.deploy_draft(auth_ctx, draft_id.into_inner()).await?;
    let matcher_config_dto = matcher_config_into_dto(result)?;
    Ok(Json(matcher_config_dto))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::auth::{AuthService, Permission};
    use crate::error::ApiError;
    use actix_web::{
        http::{header, StatusCode},
        test, App,
    };
    use async_trait::async_trait;
    use std::collections::BTreeMap;
    use std::sync::Arc;
    use tornado_engine_api_dto::auth::Auth;
    use tornado_engine_matcher::config::MatcherConfig;

    struct TestApiHandler {}

    #[async_trait]
    impl ConfigApiHandler for TestApiHandler {
        async fn get_current_config(&self) -> Result<MatcherConfig, ApiError> {
            Ok(MatcherConfig::Ruleset { name: "ruleset".to_owned(), rules: vec![] })
        }

        async fn reload_configuration(&self) -> Result<MatcherConfig, ApiError> {
            Ok(MatcherConfig::Ruleset { name: "ruleset_new".to_owned(), rules: vec![] })
        }
    }

    fn auth_service() -> AuthService {
        let mut permission_roles_map = BTreeMap::new();
        permission_roles_map.insert(Permission::ConfigEdit, vec!["edit".to_owned()]);
        permission_roles_map
            .insert(Permission::ConfigView, vec!["edit".to_owned(), "view".to_owned()]);

        AuthService::new(Arc::new(permission_roles_map))
    }

    #[actix_rt::test]
    async fn current_config_should_return_status_code_unauthorized_if_no_token(
    ) -> Result<(), ApiError> {
        // Arrange
        let mut srv = test::init_service(App::new().service(build_config_endpoints(ApiData {
            auth: auth_service(),
            api: ConfigApi::new(TestApiHandler {}),
        })))
        .await;

        // Act
        let request = test::TestRequest::get().uri("/v1/config/current").to_request();

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
            auth: auth_service(),
            api: ConfigApi::new(TestApiHandler {}),
        })))
        .await;

        // Act
        let request = test::TestRequest::get()
            .header(
                header::AUTHORIZATION,
                AuthService::auth_to_token_header(&Auth::new("user", vec![""]))?,
            )
            .uri("/v1/config/current")
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
            auth: auth_service(),
            api: ConfigApi::new(TestApiHandler {}),
        })))
        .await;

        // Act
        let request = test::TestRequest::get()
            .header(
                header::AUTHORIZATION,
                AuthService::auth_to_token_header(&Auth::new("user", vec!["edit"]))?,
            )
            .uri("/v1/config/current")
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
            auth: auth_service(),
            api: ConfigApi::new(TestApiHandler {}),
        })))
        .await;

        // Act
        let request = test::TestRequest::get()
            .header(
                header::AUTHORIZATION,
                AuthService::auth_to_token_header(&Auth::new("user", vec!["edit"]))?,
            )
            .uri("/v1/config/current")
            .to_request();

        // Assert
        let dto: tornado_engine_api_dto::config::MatcherConfigDto =
            test::read_response_json(&mut srv, request).await;

        assert_eq!(
            tornado_engine_api_dto::config::MatcherConfigDto::Ruleset {
                name: "ruleset".to_owned(),
                rules: vec![]
            },
            dto
        );

        Ok(())
    }

    #[actix_rt::test]
    async fn should_return_the_reloaded_matcher_config() -> Result<(), ApiError> {
        // Arrange
        let mut srv = test::init_service(App::new().service(build_config_endpoints(ApiData {
            auth: auth_service(),
            api: ConfigApi::new(TestApiHandler {}),
        })))
        .await;

        // Act
        let request = test::TestRequest::post()
            .header(
                header::AUTHORIZATION,
                AuthService::auth_to_token_header(&Auth::new("user", vec!["edit"]))?,
            )
            .uri("/v1/config/deploy/1")
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
}
