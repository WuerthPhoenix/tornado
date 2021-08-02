use crate::model::ApiData;
use crate::runtime_config::api::{RuntimeConfigApi, RuntimeConfigApiHandler};
use actix_web::web::{Data, Json};
use actix_web::{web, HttpRequest, Scope};
use log::*;
use tornado_engine_api_dto::runtime_config::LoggerConfigDto;

pub fn build_runtime_config_endpoints<A: RuntimeConfigApiHandler + 'static>(
    data: ApiData<RuntimeConfigApi<A>>,
) -> Scope {
    web::scope("/v1_beta/runtime_config").app_data(Data::new(data)).service(
        web::resource("/logger")
            .route(web::get().to(get_current_logger_configuration::<A>))
            .route(web::post().to(set_current_logger_configuration::<A>)),
    )
}

async fn get_current_logger_configuration<A: RuntimeConfigApiHandler + 'static>(
    req: HttpRequest,
    data: Data<ApiData<RuntimeConfigApi<A>>>,
) -> actix_web::Result<Json<LoggerConfigDto>> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req)?;
    let result = data.api.get_logger_configuration(auth_ctx).await?;
    Ok(Json(result))
}

async fn set_current_logger_configuration<A: RuntimeConfigApiHandler + 'static>(
    req: HttpRequest,
    data: Data<ApiData<RuntimeConfigApi<A>>>,
    body: Json<LoggerConfigDto>,
) -> actix_web::Result<Json<()>> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req)?;
    let result = data.api.set_logger_configuration(auth_ctx, body.into_inner()).await?;
    Ok(Json(result))
}

#[cfg(test)]
mod test {
    use crate::auth::{test_auth_service, AuthService};
    use crate::error::ApiError;
    use crate::model::ApiData;
    use crate::runtime_config::api::test::TestRuntimeConfigApiHandler;
    use crate::runtime_config::api::RuntimeConfigApi;
    use crate::runtime_config::web::build_runtime_config_endpoints;
    use actix_web::{http::header, http::StatusCode, test, App};
    use tornado_engine_api_dto::auth::Auth;
    use tornado_engine_api_dto::runtime_config::LoggerConfigDto;

    #[actix_rt::test]
    async fn current_logger_config_should_return_status_code_unauthorized_if_no_token(
    ) -> Result<(), ApiError> {
        // Arrange
        let mut srv =
            test::init_service(App::new().service(build_runtime_config_endpoints(ApiData {
                auth: test_auth_service(),
                api: RuntimeConfigApi::new(TestRuntimeConfigApiHandler {}),
            })))
            .await;

        // Act
        let request = test::TestRequest::get()
            .insert_header((header::CONTENT_TYPE, "application/json"))
            .uri("/v1_beta/runtime_config/logger")
            .to_request();

        let response = test::call_service(&mut srv, request).await;

        // Assert
        assert_eq!(StatusCode::UNAUTHORIZED, response.status());
        Ok(())
    }

    #[actix_rt::test]
    async fn get_current_logger_config_should_return_config() -> Result<(), ApiError> {
        // Arrange
        let mut srv =
            test::init_service(App::new().service(build_runtime_config_endpoints(ApiData {
                auth: test_auth_service(),
                api: RuntimeConfigApi::new(TestRuntimeConfigApiHandler {}),
            })))
            .await;

        // Act
        let request = test::TestRequest::get()
            .insert_header((header::CONTENT_TYPE, "application/json"))
            .insert_header((
                header::AUTHORIZATION,
                AuthService::auth_to_token_header(&Auth::new("user", vec!["runtime_config_view"]))
                    .unwrap(),
            ))
            .uri("/v1_beta/runtime_config/logger")
            .to_request();

        let dto: tornado_engine_api_dto::runtime_config::LoggerConfigDto =
            test::read_response_json(&mut srv, request).await;

        // Assert
        assert!(!dto.level.is_empty());
        Ok(())
    }

    #[actix_rt::test]
    async fn set_current_logger_config_should_set_config() -> Result<(), ApiError> {
        // Arrange
        let mut srv =
            test::init_service(App::new().service(build_runtime_config_endpoints(ApiData {
                auth: test_auth_service(),
                api: RuntimeConfigApi::new(TestRuntimeConfigApiHandler {}),
            })))
            .await;

        // Act
        let request = test::TestRequest::post()
            .insert_header((header::CONTENT_TYPE, "application/json"))
            .insert_header((
                header::AUTHORIZATION,
                AuthService::auth_to_token_header(&Auth::new("user", vec!["runtime_config_edit"]))
                    .unwrap(),
            ))
            .set_payload(
                serde_json::to_string(&LoggerConfigDto { level: "info".to_owned() }).unwrap(),
            )
            .uri("/v1_beta/runtime_config/logger")
            .to_request();

        let response = test::call_service(&mut srv, request).await;

        // Assert
        assert_eq!(StatusCode::OK, response.status());
        Ok(())
    }
}
