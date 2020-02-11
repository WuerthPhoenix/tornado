use self::handler::ApiHandler;
use crate::convert::config::matcher_config_into_dto;
use crate::convert::event::{dto_into_send_event_request, processed_event_into_dto};
use actix_web::web::{Data, Json};
use actix_web::{web, Scope};
use log::*;
use std::ops::Deref;
use tornado_engine_api_dto::config::MatcherConfigDto;
use tornado_engine_api_dto::event::{ProcessedEventDto, SendEventRequestDto};

pub mod handler;

pub fn new_endpoints<T: ApiHandler + 'static>(scope: Scope, api_handler: T) -> Scope {
    scope
        .data(api_handler)
        .service(web::resource("/config").route(web::get().to(get_config::<T>)))
        .service(web::resource("/send_event").route(web::post().to(send_event::<T>)))
}

/*
async fn web_block_json<I, F>(f: F) -> Result<Json<I>, ApiError>
where
    F: FnOnce() -> Result<I, ApiError> + Send + 'static,
    I: Send + 'static,
{
    actix_web::web::block(f)
        .await
        .map_err(|err| match err {
            BlockingError::Error(e) => e,
            _ => ApiError::InternalServerError { cause: format!("{}", err) },
        })
        .map(Json)
}
*/

async fn get_config<T: ApiHandler + 'static>(
    api_handler: Data<T>,
) -> actix_web::Result<Json<MatcherConfigDto>> {
    debug!("API - received get_config request");
    let matcher_config = api_handler.get_config().await?;

    let matcher_config_dto = matcher_config_into_dto(matcher_config)?;
    Ok(Json(matcher_config_dto))
}

async fn send_event<T: ApiHandler + 'static>(
    api_handler: Data<T>,
    body: Json<SendEventRequestDto>,
) -> actix_web::Result<Json<ProcessedEventDto>> {
    if log_enabled!(Level::Debug) {
        let json_string = serde_json::to_string(body.deref()).unwrap();
        debug!("API - received send_event request: {}", json_string);
    }

    let send_event_request = dto_into_send_event_request(body.into_inner())?;
    let processed_event = api_handler.send_event(send_event_request).await?;
    Ok(Json(processed_event_into_dto(processed_event)?))

    /*
    web_block_json(move || {
        let send_event_request = dto_into_send_event_request(body.into_inner())?;
        let processed_event = api_handler.send_event(send_event_request)?;
        Ok(processed_event_into_dto(processed_event)?)
    })
    .await
    */
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::api::handler::SendEventRequest;
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
    impl ApiHandler for TestApiHandler {
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
            App::new().service(new_endpoints(web::scope("/api"), TestApiHandler {})),
        )
        .await;

        // Act
        let request = test::TestRequest::get()
            .uri("/api/config")
            //.header(header::CONTENT_TYPE, "application/json")
            //.set_payload(payload)
            .to_request();

        let response = test::call_service(&mut srv, request).await;

        // Assert
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[actix_rt::test]
    async fn should_return_the_matcher_config() {
        // Arrange
        let mut srv = test::init_service(
            App::new().service(new_endpoints(web::scope("/api"), TestApiHandler {})),
        )
        .await;

        // Act
        let request = test::TestRequest::get()
            .uri("/api/config")
            //.header(header::CONTENT_TYPE, "application/json")
            //.set_payload(payload)
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
    }

    #[actix_rt::test]
    async fn should_return_the_processed_event() {
        // Arrange
        let mut srv = test::init_service(
            App::new().service(new_endpoints(web::scope("/api"), TestApiHandler {})),
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
