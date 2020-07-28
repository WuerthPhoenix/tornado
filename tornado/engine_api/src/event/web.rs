use crate::event::api::{EventApi, EventApiHandler};
use crate::event::convert::{dto_into_send_event_request, processed_event_into_dto};
use crate::model::ApiData;
use actix_web::web::{Data, Json};
use actix_web::{web, HttpRequest, Scope};
use log::*;
use std::ops::Deref;
use tornado_engine_api_dto::event::{ProcessedEventDto, SendEventRequestDto};

pub fn build_event_endpoints<T: EventApiHandler + 'static>(data: ApiData<EventApi<T>>) -> Scope {
    web::scope("/v1_beta/event")
        .data(data)
        .service(web::resource("/current/send").route(web::post().to(send_event::<T>)))
}

async fn send_event<T: EventApiHandler + 'static>(
    req: HttpRequest,
    data: Data<ApiData<EventApi<T>>>,
    body: Json<SendEventRequestDto>,
) -> actix_web::Result<Json<ProcessedEventDto>> {
    if log_enabled!(Level::Debug) {
        debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
        let json_string = serde_json::to_string(body.deref()).unwrap();
        debug!("API - received send_event request: {}", json_string);
    }

    let auth_ctx = data.auth.auth_from_request(&req)?;
    let send_event_request = dto_into_send_event_request(body.into_inner())?;
    let processed_event =
        data.api.send_event_to_current_config(auth_ctx, send_event_request).await?;
    Ok(Json(processed_event_into_dto(processed_event)?))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::auth::{test_auth_service, AuthService};
    use crate::error::ApiError;
    use crate::event::api::SendEventRequest;
    use actix_web::{http::header, test, App};
    use async_trait::async_trait;
    use std::collections::HashMap;
    use tornado_common_api::Value;
    use tornado_engine_api_dto::auth::Auth;
    use tornado_engine_api_dto::event::{EventDto, ProcessType, SendEventRequestDto};
    use tornado_engine_matcher::model::{ProcessedEvent, ProcessedNode, ProcessedRules};

    struct TestApiHandler {}

    #[async_trait]
    impl EventApiHandler for TestApiHandler {
        async fn send_event_to_current_config(
            &self,
            event: SendEventRequest,
        ) -> Result<ProcessedEvent, ApiError> {
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
    async fn should_return_the_processed_event() {
        // Arrange
        let mut srv = test::init_service(App::new().service(build_event_endpoints(ApiData {
            auth: test_auth_service(),
            api: EventApi::new(TestApiHandler {}),
        })))
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
            .uri("/v1_beta/event/current/send")
            .header(header::CONTENT_TYPE, "application/json")
            .header(
                header::AUTHORIZATION,
                AuthService::auth_to_token_header(&Auth::new("user", vec!["edit"])).unwrap(),
            )
            .set_payload(serde_json::to_string(&send_event_request).unwrap())
            .to_request();

        // Assert
        let dto: tornado_engine_api_dto::event::ProcessedEventDto =
            test::read_response_json(&mut srv, request).await;
        assert_eq!("my_test_event", dto.event.event_type);
    }
}
