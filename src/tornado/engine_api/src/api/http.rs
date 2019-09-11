use crate::api::handler::ApiHandler;
use crate::convert::config::matcher_config_into_dto;
use crate::convert::event::{dto_into_send_event_request, processed_event_into_dto};
use crate::error::ApiError;
use actix_web::web::Json;
use actix_web::{Error as AWError, HttpRequest, HttpResponse};
use futures::future::FutureResult;
use futures::Future;
use log::*;
use std::ops::Deref;
use std::sync::Arc;
use tornado_engine_api_dto::event::SendEventRequestDto;

/// The HttpHandler wraps an ApiHandler hiding the low level HTTP Request details
/// and handling the DTOs conversions.
pub struct HttpHandler<T: ApiHandler> {
    pub api_handler: Arc<T>,
}

impl<T: ApiHandler> Clone for HttpHandler<T> {
    fn clone(&self) -> Self {
        HttpHandler { api_handler: self.api_handler.clone() }
    }
}

impl<T: ApiHandler> HttpHandler<T> {
    pub fn get_config(
        &self,
        _req: HttpRequest,
    ) -> impl Future<Item = HttpResponse, Error = AWError> {
        debug!("API - received get_config request");
        self.api_handler.get_config().map_err(AWError::from).and_then(|matcher_config| {
            match matcher_config_into_dto(matcher_config) {
                Ok(dto) => HttpResponse::Ok().json(dto),
                Err(err) => {
                    error!("Cannot convert the MatcherConfig into a DTO. Err: {}", err);
                    HttpResponse::InternalServerError().finish()
                }
            }
        })
    }

    pub fn send_event(
        &self,
        _req: HttpRequest,
        body: Json<SendEventRequestDto>,
    ) -> impl Future<Item = HttpResponse, Error = AWError> {
        if log_enabled!(Level::Debug) {
            let json_string = serde_json::to_string(body.deref()).unwrap();
            debug!("API - received send_event request: {}", json_string);
        }

        let api_handler = self.api_handler.clone();

        // Futures chaining.
        // The chain starts with a Result (returned by dto_into_send_event_request(body.into_inner()))
        // converted into a Future and the chained to the api_handler call.
        FutureResult::from(dto_into_send_event_request(body.into_inner()))
            .map_err(ApiError::from)
            .and_then(move |send_event_request| api_handler.send_event(send_event_request))
            .map_err(AWError::from)
            .and_then(|processed_event| match processed_event_into_dto(processed_event) {
                Ok(dto) => HttpResponse::Ok().json(dto),
                Err(err) => {
                    error!("Cannot convert the processed_event into a DTO. Err: {}", err);
                    HttpResponse::InternalServerError().finish()
                }
            })
    }
}
