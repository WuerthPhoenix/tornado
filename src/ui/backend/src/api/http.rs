use crate::api::handler::ApiHandler;
use crate::convert::config::matcher_config_into_dto;
use actix_web::{Error as AWError, HttpRequest, HttpResponse};
use futures::Future;
use std::sync::Arc;
use tornado_common_api::Event;
use log::*;
use crate::convert::event::{processed_event_into_dto, dto_into_send_event_request};
use actix_web::web::Json;
use dto::event::SendEventRequestDto;

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

    pub fn get_config(&self, _req: HttpRequest) -> impl Future<Item = HttpResponse, Error = AWError> {
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

    pub fn send_event(&self, _req: HttpRequest, body: Json<SendEventRequestDto>) -> impl Future<Item = HttpResponse, Error = AWError> {
        debug!("API - received send_event request");
        //let event = Event::new("fake_event");

        self.api_handler.send_event(dto_into_send_event_request(body.into_inner())?).map_err(AWError::from).and_then(|processed_event| {
            match processed_event_into_dto(processed_event) {
                Ok(dto) => HttpResponse::Ok().json(dto),
                Err(err) => {
                    error!("Cannot convert the processed_event into a DTO. Err: {}", err);
                    HttpResponse::InternalServerError().finish()
                }
            }
        })

    }
}
