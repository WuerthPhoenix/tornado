use crate::auth::auth_v2::{AuthContextV2, AuthServiceV2};
use crate::error::ApiError;
use crate::event::api::{EventApiHandler, SendEventRequest};
use crate::event::api_v2::EventApiV2;
use crate::event::convert::{dto_into_send_event_request, processed_event_into_dto};
use crate::model::ApiDataV2;
use actix_web::web::{Data, Json, Path};
use actix_web::{web, HttpRequest, Scope};
use log::*;
use serde::Deserialize;
use std::ops::Deref;
use tornado_engine_api_dto::event::{ProcessedEventDto, SendEventRequestDto};
use tornado_engine_matcher::config::MatcherConfigEditor;

pub fn build_event_v2_endpoints<
    T: EventApiHandler + 'static,
    CM: MatcherConfigEditor + ?Sized + 'static,
>(
    data: ApiDataV2<EventApiV2<T, CM>>,
) -> Scope {
    web::scope("/event")
        .app_data(Data::new(data))
        .service(
            web::resource("/active/{param_auth}")
                .route(web::post().to(send_event_to_current_config_v2::<T, CM>)),
        )
        .service(
            web::resource("/drafts/{param_auth}/{draft_id}")
                .route(web::post().to(send_event_to_draft_v2::<T, CM>)),
        )
}

#[derive(Deserialize)]
struct EndpointParamAuthPath {
    param_auth: String,
}

#[derive(Deserialize)]
struct AuthAndDraftId {
    param_auth: String,
    draft_id: String,
}

fn prepare_data_for_send_event_v2<'a>(
    req: &HttpRequest,
    auth: &'a AuthServiceV2,
    param_auth: &str,
    body: Json<SendEventRequestDto>,
) -> Result<(AuthContextV2<'a>, SendEventRequest), ApiError> {
    let auth_ctx = auth.auth_from_request(req, param_auth)?;
    let send_event_request = dto_into_send_event_request(body.into_inner())?;
    Ok((auth_ctx, send_event_request))
}

async fn send_event_to_current_config_v2<
    T: EventApiHandler + 'static,
    CM: MatcherConfigEditor + ?Sized + 'static,
>(
    req: HttpRequest,
    data: Data<ApiDataV2<EventApiV2<T, CM>>>,
    params: Path<EndpointParamAuthPath>,
    body: Json<SendEventRequestDto>,
    _param_auth: Path<String>,
) -> actix_web::Result<Json<ProcessedEventDto>> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    if log_enabled!(Level::Debug) {
        let json_string = serde_json::to_string(body.deref()).unwrap();
        debug!("API - received send_event_to_current_config_v2 request: {}", json_string);
    }

    let (auth_ctx, send_event_request) =
        prepare_data_for_send_event_v2(&req, &data.auth, &params.param_auth, body)?;

    let processed_event =
        data.api.send_event_to_current_config(auth_ctx, send_event_request).await?;
    Ok(Json(processed_event_into_dto(processed_event)?))
}

async fn send_event_to_draft_v2<
    T: EventApiHandler + 'static,
    CM: MatcherConfigEditor + ?Sized + 'static,
>(
    req: HttpRequest,
    data: Data<ApiDataV2<EventApiV2<T, CM>>>,
    params: Path<AuthAndDraftId>,
    body: Json<SendEventRequestDto>,
) -> actix_web::Result<Json<ProcessedEventDto>> {
    debug!("HttpRequest method [{}] path [{}]", req.method(), req.path());
    if log_enabled!(Level::Debug) {
        let json_string = serde_json::to_string(body.deref()).unwrap();
        debug!("API - received send_event_to_draft_v2 request: {}", json_string);
    }

    let (auth_ctx, send_event_request) =
        prepare_data_for_send_event_v2(&req, &data.auth, &params.param_auth, body)?;
    let processed_event =
        data.api.send_event_to_draft(auth_ctx, &params.draft_id, send_event_request).await?;
    Ok(Json(processed_event_into_dto(processed_event)?))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::auth::auth_v2::test::test_auth_service_v2;
    use crate::event::api::test::{TestApiHandler, TestConfigManager};
    use actix_web::{http::header, test, App};
    use std::collections::HashMap;
    use std::sync::Arc;
    use tornado_engine_api_dto::auth_v2::{AuthHeaderV2, Authorization};
    use tornado_engine_api_dto::event::{EventDto, ProcessType, SendEventRequestDto};

    fn get_something() -> HashMap<String, serde_json::Value> {
        let mut something = HashMap::new();
        something.insert(
            "something".to_owned(),
            serde_json::Value::String(format!("{}", rand::random::<usize>())),
        );
        something
    }

    #[actix_rt::test]
    async fn should_send_event_to_current_config_v2() {
        // Arrange
        let srv = test::init_service(App::new().service(build_event_v2_endpoints(ApiDataV2 {
            auth: test_auth_service_v2(),
            api: EventApiV2::new(TestApiHandler {}, Arc::new(TestConfigManager {})),
        })))
        .await;

        let metadata = get_something();

        let send_event_request = SendEventRequestDto {
            event: EventDto {
                event_type: "my_test_event".to_owned(),
                payload: HashMap::new(),
                metadata: metadata.clone(),
                created_ms: 0,
                iterator: None,
            },
            process_type: ProcessType::SkipActions,
        };

        // Act
        let mut auths = HashMap::new();
        auths.insert(
            "auth1".to_owned(),
            Authorization { path: vec!["root".to_owned()], roles: vec!["view".to_owned()] },
        );
        let request = test::TestRequest::post()
            .uri("/event/active/auth1")
            .insert_header((header::CONTENT_TYPE, "application/json"))
            .insert_header((
                header::AUTHORIZATION,
                AuthServiceV2::auth_to_token_header(&AuthHeaderV2 {
                    user: "admin".to_string(),
                    auths,
                    preferences: None,
                })
                .unwrap(),
            ))
            .set_payload(serde_json::to_string(&send_event_request).unwrap())
            .to_request();

        // Assert

        let resp = test::call_service(&srv, request).await;

        assert_eq!(200, resp.status());

        let dto: tornado_engine_api_dto::event::ProcessedEventDto =
            test::read_body_json(resp).await;

        assert_eq!("my_test_event", dto.event.event_type);
        assert_eq!(metadata, dto.event.metadata);
    }

    #[actix_rt::test]
    async fn send_event_to_current_config_v2_should_return_unauthorized_if_path_not_in_auths() {
        // Arrange
        let srv = test::init_service(App::new().service(build_event_v2_endpoints(ApiDataV2 {
            auth: test_auth_service_v2(),
            api: EventApiV2::new(TestApiHandler {}, Arc::new(TestConfigManager {})),
        })))
        .await;

        let send_event_request = SendEventRequestDto {
            event: EventDto {
                event_type: "my_test_event".to_owned(),
                payload: HashMap::new(),
                metadata: Default::default(),
                created_ms: 0,
                iterator: None,
            },
            process_type: ProcessType::SkipActions,
        };

        // Act
        let mut auths = HashMap::new();
        auths.insert(
            "auth1".to_owned(),
            Authorization { path: vec!["root".to_owned()], roles: vec!["view".to_owned()] },
        );
        let request = test::TestRequest::post()
            .uri("/event/active/auth2")
            .insert_header((header::CONTENT_TYPE, "application/json"))
            .insert_header((
                header::AUTHORIZATION,
                AuthServiceV2::auth_to_token_header(&AuthHeaderV2 {
                    user: "admin".to_string(),
                    auths,
                    preferences: None,
                })
                .unwrap(),
            ))
            .set_payload(serde_json::to_string(&send_event_request).unwrap())
            .to_request();

        // Assert
        let resp = test::call_service(&srv, request).await;
        //println!("resp: [{:?}]", resp);

        assert_eq!(401, resp.status());
    }

    #[actix_rt::test]
    async fn should_send_event_to_draft_v2() {
        // Arrange
        let srv = test::init_service(App::new().service(build_event_v2_endpoints(ApiDataV2 {
            auth: test_auth_service_v2(),
            api: EventApiV2::new(TestApiHandler {}, Arc::new(TestConfigManager {})),
        })))
        .await;

        let metadata = get_something();
        let send_event_request = SendEventRequestDto {
            event: EventDto {
                event_type: "my_test_event_for_draft".to_owned(),
                payload: HashMap::new(),
                metadata: metadata.clone(),
                created_ms: 0,
                iterator: None,
            },
            process_type: ProcessType::SkipActions,
        };

        // Act
        let mut auths = HashMap::new();
        auths.insert(
            "auth1".to_owned(),
            Authorization { path: vec!["root".to_owned()], roles: vec!["view".to_owned()] },
        );
        let request = test::TestRequest::post()
            .uri("/event/drafts/auth1/a123")
            .insert_header((header::CONTENT_TYPE, "application/json"))
            .insert_header((
                header::AUTHORIZATION,
                AuthServiceV2::auth_to_token_header(&AuthHeaderV2 {
                    user: "OWNER".to_string(),
                    auths,
                    preferences: None,
                })
                .unwrap(),
            ))
            .set_payload(serde_json::to_string(&send_event_request).unwrap())
            .to_request();

        // Assert
        let resp = test::call_service(&srv, request).await;
        assert_eq!(200, resp.status());

        let dto: tornado_engine_api_dto::event::ProcessedEventDto =
            test::read_body_json(resp).await;

        assert_eq!("my_test_event_for_draft", dto.event.event_type);
        assert_eq!(metadata, dto.event.metadata);
    }
}
