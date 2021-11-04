use crate::event::api::{EventApi, EventApiHandler};
use crate::event::convert::{dto_into_send_event_request, processed_event_into_dto};
use crate::model::ApiData;
use actix_web::web::{Data, Json, Path};
use actix_web::{web, HttpRequest, Scope};
use log::*;
use tornado_engine_matcher::config::fs::ROOT_NODE_NAME;
use tornado_engine_matcher::config::operation::NodeFilter;
use std::collections::HashMap;
use std::ops::Deref;
use tornado_engine_api_dto::event::{ProcessedEventDto, SendEventRequestDto};
use tornado_engine_matcher::config::MatcherConfigEditor;

pub fn build_event_endpoints<T: EventApiHandler + 'static, CM: MatcherConfigEditor + 'static>(
    data: ApiData<EventApi<T, CM>>,
) -> Scope {
    web::scope("/v1_beta/event")
        .app_data(Data::new(data))
        .service(
            web::resource("/current/send")
                .route(web::post().to(send_event_to_current_config::<T, CM>)),
        )
        .service(
            web::resource("/drafts/{draft_id}/send")
                .route(web::post().to(send_event_to_draft::<T, CM>)),
        )
}

async fn send_event_to_current_config<
    T: EventApiHandler + 'static,
    CM: MatcherConfigEditor + 'static,
>(
    req: HttpRequest,
    data: Data<ApiData<EventApi<T, CM>>>,
    body: Json<SendEventRequestDto>,
) -> actix_web::Result<Json<ProcessedEventDto>> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());

    if log_enabled!(Level::Debug) {
        let json_string = serde_json::to_string(body.deref()).unwrap();
        debug!("API - received send_event_to_current_config request: {}", json_string);
    }

    let auth_ctx = data.auth.auth_from_request(&req)?;
    let send_event_request = dto_into_send_event_request(body.into_inner())?;
    let processed_event =
        data.api.send_event_to_current_config(auth_ctx, send_event_request).await?;
    Ok(Json(processed_event_into_dto(processed_event)?))
}

async fn send_event_to_draft<T: EventApiHandler + 'static, CM: MatcherConfigEditor + 'static>(
    req: HttpRequest,
    data: Data<ApiData<EventApi<T, CM>>>,
    draft_id: Path<String>,
    body: Json<SendEventRequestDto>,
) -> actix_web::Result<Json<ProcessedEventDto>> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    let draft_id = draft_id.into_inner();

    if log_enabled!(Level::Debug) {
        let json_string = serde_json::to_string(body.deref()).unwrap();
        debug!("API - received send_event_to_draft [{}] request: {}", draft_id, json_string);
    }

    let auth_ctx = data.auth.auth_from_request(&req)?;
    let send_event_request = dto_into_send_event_request(body.into_inner())?;
    let processed_event =
        data.api.send_event_to_draft(auth_ctx, &draft_id, send_event_request).await?;
    Ok(Json(processed_event_into_dto(processed_event)?))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::auth::{test_auth_service, AuthService};
    use crate::event::api::test::{TestApiHandler, TestConfigManager, DRAFT_OWNER_ID};
    use actix_web::{http::header, test, App};
    use std::collections::HashMap;
    use std::sync::Arc;
    use tornado_engine_api_dto::auth::Auth;
    use tornado_engine_api_dto::event::{EventDto, ProcessType, SendEventRequestDto};

    #[actix_rt::test]
    async fn should_send_event_to_current_config() {
        // Arrange
        let mut srv = test::init_service(App::new().service(build_event_endpoints(ApiData {
            auth: test_auth_service(),
            api: EventApi::new(TestApiHandler {}, Arc::new(TestConfigManager {})),
        })))
        .await;

        let send_event_request = SendEventRequestDto {
            event: EventDto {
                event_type: "my_test_event".to_owned(),
                payload: HashMap::new(),
                created_ms: 0,
                trace_id: Some("my_trace_id".to_owned()),
            },
            process_type: ProcessType::SkipActions,
        };

        // Act
        let request = test::TestRequest::post()
            .uri("/v1_beta/event/current/send")
            .insert_header((header::CONTENT_TYPE, "application/json"))
            .insert_header((
                header::AUTHORIZATION,
                AuthService::auth_to_token_header(&Auth::new("user", vec!["edit"])).unwrap(),
            ))
            .set_payload(serde_json::to_string(&send_event_request).unwrap())
            .to_request();

        // Assert
        let dto: tornado_engine_api_dto::event::ProcessedEventDto =
            test::read_response_json(&mut srv, request).await;
        assert_eq!("my_test_event", dto.event.event_type);
        assert_eq!(Some("my_trace_id".to_owned()), dto.event.trace_id);
    }

    #[actix_rt::test]
    async fn should_send_event_to_draft() {
        // Arrange
        let mut srv = test::init_service(App::new().service(build_event_endpoints(ApiData {
            auth: test_auth_service(),
            api: EventApi::new(TestApiHandler {}, Arc::new(TestConfigManager {})),
        })))
        .await;

        let send_event_request = SendEventRequestDto {
            event: EventDto {
                event_type: "my_test_event_for_draft".to_owned(),
                payload: HashMap::new(),
                created_ms: 0,
                trace_id: None,
            },
            process_type: ProcessType::SkipActions,
        };
        // Act
        let request = test::TestRequest::post()
            .uri("/v1_beta/event/drafts/a125/send")
            .insert_header((header::CONTENT_TYPE, "application/json"))
            .insert_header((
                header::AUTHORIZATION,
                AuthService::auth_to_token_header(&Auth::new(DRAFT_OWNER_ID, vec!["edit"]))
                    .unwrap(),
            ))
            .set_payload(serde_json::to_string(&send_event_request).unwrap())
            .to_request();

        // Assert
        let dto: tornado_engine_api_dto::event::ProcessedEventDto =
            test::read_response_json(&mut srv, request).await;
        assert_eq!("my_test_event_for_draft", dto.event.event_type);
        assert!(dto.event.trace_id.is_some());
    }
}
