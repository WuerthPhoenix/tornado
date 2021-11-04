use crate::error::ApiError;
use crate::model::ApiData;
use crate::runtime_config::api::{RuntimeConfigApi, RuntimeConfigApiHandler};
use actix_web::web::{Data, Json};
use actix_web::{web, HttpRequest, Scope};
use ajars::actix_web::ActixWebHandler;
use log::*;
use tornado_engine_api_dto::runtime_config::{
    LoggerConfigDto, SetApmPriorityConfigurationRequestDto, SetLoggerApmRequestDto,
    SetLoggerLevelRequestDto, SetLoggerStdoutRequestDto, SetStdoutPriorityConfigurationRequestDto,
    SET_APM_PRIORITY_CONFIG_REST, SET_STDOUT_PRIORITY_CONFIG_REST,
};

pub const RUNTIME_CONFIG_ENDPOINT_V1_BASE: &str = "/v1_beta/runtime_config";

pub fn build_runtime_config_endpoints<A: RuntimeConfigApiHandler + 'static>(
    data: ApiData<RuntimeConfigApi<A>>,
) -> Scope {
    web::scope(RUNTIME_CONFIG_ENDPOINT_V1_BASE)
        .app_data(Data::new(data))
        .service(
            web::resource("/logger/level").route(web::post().to(set_current_logger_level::<A>)),
        )
        .service(web::resource("/logger/stdout").route(web::post().to(set_stdout::<A>)))
        .service(web::resource("/logger/apm").route(web::post().to(set_apm::<A>)))
        .service(SET_APM_PRIORITY_CONFIG_REST.to(set_apm_priority_config::<A>))
        .service(SET_STDOUT_PRIORITY_CONFIG_REST.to(set_stdout_priority_config::<A>))
        .service(
            web::resource("/logger").route(web::get().to(get_current_logger_configuration::<A>)),
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

async fn set_current_logger_level<A: RuntimeConfigApiHandler + 'static>(
    req: HttpRequest,
    data: Data<ApiData<RuntimeConfigApi<A>>>,
    body: Json<SetLoggerLevelRequestDto>,
) -> actix_web::Result<Json<()>> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req)?;
    let result = data.api.set_logger_level(auth_ctx, body.into_inner()).await?;
    Ok(Json(result))
}

async fn set_apm<A: RuntimeConfigApiHandler + 'static>(
    req: HttpRequest,
    data: Data<ApiData<RuntimeConfigApi<A>>>,
    body: Json<SetLoggerApmRequestDto>,
) -> actix_web::Result<Json<()>> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req)?;
    let result = data.api.set_apm_enabled(auth_ctx, body.into_inner()).await?;
    Ok(Json(result))
}

async fn set_stdout<A: RuntimeConfigApiHandler + 'static>(
    req: HttpRequest,
    data: Data<ApiData<RuntimeConfigApi<A>>>,
    body: Json<SetLoggerStdoutRequestDto>,
) -> actix_web::Result<Json<()>> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req)?;
    let result = data.api.set_stdout_enabled(auth_ctx, body.into_inner()).await?;
    Ok(Json(result))
}

async fn set_apm_priority_config<A: RuntimeConfigApiHandler + 'static>(
    body: SetApmPriorityConfigurationRequestDto,
    req: HttpRequest,
    data: Data<ApiData<RuntimeConfigApi<A>>>,
) -> Result<(), ApiError> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req)?;
    data.api.set_apm_priority_configuration(auth_ctx, body).await
}

async fn set_stdout_priority_config<A: RuntimeConfigApiHandler + 'static>(
    body: SetStdoutPriorityConfigurationRequestDto,
    req: HttpRequest,
    data: Data<ApiData<RuntimeConfigApi<A>>>,
) -> Result<(), ApiError> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req)?;
    data.api.set_stdout_priority_configuration(auth_ctx, body).await
}

#[cfg(test)]
mod test {
    use crate::auth::AuthService;
    use crate::auth::test::test_auth_service;
    use crate::error::ApiError;
    use crate::model::ApiData;
    use crate::runtime_config::api::test::TestRuntimeConfigApiHandler;
    use crate::runtime_config::api::RuntimeConfigApi;
    use crate::runtime_config::web::build_runtime_config_endpoints;
    use actix_web::{http::header, http::StatusCode, test, App};
    use tornado_engine_api_dto::auth::Auth;
    use tornado_engine_api_dto::runtime_config::{
        SetLoggerApmRequestDto, SetLoggerLevelRequestDto, SetLoggerStdoutRequestDto,
    };

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
    async fn set_current_logger_level_should_set_config() -> Result<(), ApiError> {
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
                serde_json::to_string(&SetLoggerLevelRequestDto { level: "info".to_owned() })
                    .unwrap(),
            )
            .uri("/v1_beta/runtime_config/logger/level")
            .to_request();

        let response = test::call_service(&mut srv, request).await;

        // Assert
        assert_eq!(StatusCode::OK, response.status());
        Ok(())
    }

    #[actix_rt::test]
    async fn set_apm_enabled_should_set_apm() -> Result<(), ApiError> {
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
            .set_payload(serde_json::to_string(&SetLoggerApmRequestDto { enabled: false }).unwrap())
            .uri("/v1_beta/runtime_config/logger/apm")
            .to_request();

        let response = test::call_service(&mut srv, request).await;

        // Assert
        assert_eq!(StatusCode::OK, response.status());
        Ok(())
    }

    #[actix_rt::test]
    async fn set_apm_enabled_should_return_unauthorized() -> Result<(), ApiError> {
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
            .set_payload(serde_json::to_string(&SetLoggerApmRequestDto { enabled: false }).unwrap())
            .uri("/v1_beta/runtime_config/logger/apm")
            .to_request();

        let response = test::call_service(&mut srv, request).await;

        // Assert
        assert_eq!(StatusCode::UNAUTHORIZED, response.status());
        Ok(())
    }

    #[actix_rt::test]
    async fn set_stdout_enabled_should_set_stdout() -> Result<(), ApiError> {
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
                serde_json::to_string(&SetLoggerStdoutRequestDto { enabled: false }).unwrap(),
            )
            .uri("/v1_beta/runtime_config/logger/stdout")
            .to_request();

        let response = test::call_service(&mut srv, request).await;

        // Assert
        assert_eq!(StatusCode::OK, response.status());
        Ok(())
    }

    #[actix_rt::test]
    async fn set_stdout_enabled_should_return_unauthorized() -> Result<(), ApiError> {
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
            .set_payload(
                serde_json::to_string(&SetLoggerStdoutRequestDto { enabled: false }).unwrap(),
            )
            .uri("/v1_beta/runtime_config/logger/stdout")
            .to_request();

        let response = test::call_service(&mut srv, request).await;

        // Assert
        assert_eq!(StatusCode::UNAUTHORIZED, response.status());
        Ok(())
    }
}
