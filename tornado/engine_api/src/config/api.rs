use actix_web::web::{Data, Json, Path};
use actix_web::{web, Scope, HttpRequest};
use log::*;
use tornado_engine_api_dto::config::MatcherConfigDto;
use crate::config::convert::{matcher_config_into_dto, dto_into_matcher_config};
use crate::config::handler::ConfigApiHandler;
use crate::model::ApiData;

pub fn build_config_endpoints(scope: Scope, data: ApiData<ConfigApiHandler>) -> Scope {
    scope
        .data(data)
        .service(
            web::scope("/v1/config")
                .service(web::resource("/drafts").route(web::get().to(get_drafts)))
                .service(web::resource("/draft").route(web::post().to(create_draft)))
                .service(web::resource("/draft/{draft_id}").route(web::post().to(create_draft))
                    .route(web::get().to(get_draft))
                    .route(web::put().to(update_draft))
                    .route(web::delete().to(delete_draft))
                )
        )
}

async fn get_drafts(
    req: HttpRequest,
    data: Data<ApiData<ConfigApiHandler>>,
) -> actix_web::Result<Json<Vec<String>>> {
     debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req)?;
    let result = data.api.get_drafts(auth_ctx).await?;
    Ok(Json(result))
}

async fn get_draft(
    req: HttpRequest,
    draft_id: Path<String>,
    data: Data<ApiData<ConfigApiHandler>>,
) -> actix_web::Result<Json<MatcherConfigDto>> {
     debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req)?;
    let result = data.api.get_draft(auth_ctx, draft_id.into_inner()).await?;
    let matcher_config_dto = matcher_config_into_dto(result)?;
    Ok(Json(matcher_config_dto))
}

async fn create_draft(
    req: HttpRequest,
    data: Data<ApiData<ConfigApiHandler>>,
) -> actix_web::Result<Json<String>> {
     debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req)?;
    let result = data.api.create_draft(auth_ctx).await?;
    Ok(Json(result))
}

async fn update_draft(
    req: HttpRequest,
    draft_id: Path<String>,
    body: Json<MatcherConfigDto>,
    data: Data<ApiData<ConfigApiHandler>>,
) -> actix_web::Result<Json<()>> {
     debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req)?;
    let config = dto_into_matcher_config(body.into_inner())?;
    let result = data.api.update_draft(auth_ctx, draft_id.into_inner(), config).await?;
    Ok(Json(result))
}

async fn delete_draft(
    req: HttpRequest,
    draft_id: Path<String>,
    data: Data<ApiData<ConfigApiHandler>>,
) -> actix_web::Result<Json<()>> {
     debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let auth_ctx = data.auth.auth_from_request(&req)?;
    let result = data.api.delete_draft(auth_ctx, draft_id.into_inner()).await?;
    Ok(Json(result))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::event::handler::SendEventRequest;
    use crate::error::ApiError;
    use actix_web::{
        http::{header, StatusCode},
        test, App,
    };
    use async_trait::async_trait;
    use std::collections::HashMap;
    use tornado_common_api::Value;
    use tornado_engine_api_dto::event::{EventDto, ProcessType, SendEventRequestDto};
    use tornado_engine_matcher::config::MatcherConfig;
    use tornado_engine_matcher::model::{ProcessedEvent, ProcessedNode, ProcessedRules};

    struct TestApiHandler {}

    #[async_trait]
    impl EventApiHandler for TestApiHandler {
        async fn get_config(&self) -> Result<MatcherConfig, ApiError> {
            Ok(MatcherConfig::Ruleset { name: "ruleset".to_owned(), rules: vec![] })
        }

        async fn send_event(&self, event: SendEventRequest) -> Result<ProcessedEvent, ApiError> {
            Ok(ProcessedEvent {
                event: event.event.into(),
                result: ProcessedNode::Ruleset {
                    name: "ruleset".to_owned(),
                    rules: ProcessedRules {
                        rules: vec![],
                        extracted_vars: Value::Map(HashMap::new()),
                    },
                },
            })
        }
    }

    #[actix_rt::test]
    async fn should_return_status_code_ok() {
        // Arrange
        let mut srv = test::init_service(
            App::new().service(build_event_endpoints(web::scope("/api"), TestApiHandler {})),
        )
            .await;

        // Act
        let request = test::TestRequest::get().uri("/api/config").to_request();

        let response = test::call_service(&mut srv, request).await;

        // Assert
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[actix_rt::test]
    async fn should_return_the_matcher_config() {
        // Arrange
        let mut srv = test::init_service(
            App::new().service(build_event_endpoints(web::scope("/api"), TestApiHandler {})),
        )
            .await;

        // Act
        let request = test::TestRequest::get().uri("/api/config").to_request();

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
    }

    #[actix_rt::test]
    async fn should_return_the_processed_event() {
        // Arrange
        let mut srv = test::init_service(
            App::new().service(build_event_endpoints(web::scope("/api"), TestApiHandler {})),
        )
            .await;

        let send_event_request = SendEventRequestDto {
            event: EventDto {
                event_type: "my_test_event".to_owned(),
                payload: HashMap::new(),
                created_ms: 0,
            },
            process_type: ProcessType::SkipActions,
        };

        // Act
        let request = test::TestRequest::post()
            .uri("/api/send_event")
            .header(header::CONTENT_TYPE, "application/json")
            .set_payload(serde_json::to_string(&send_event_request).unwrap())
            .to_request();

        // Assert
        let dto: tornado_engine_api_dto::event::ProcessedEventDto =
            test::read_response_json(&mut srv, request).await;
        assert_eq!("my_test_event", dto.event.event_type);
    }
}
