use crate::api::handler::ApiHandler;
use crate::convert::config::matcher_config_to_dto;
use actix_web::{Error as AWError, HttpRequest, HttpResponse};
use futures::Future;
use std::sync::Arc;
use tornado_common_api::Event;
use log::*;

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
            match matcher_config_to_dto(matcher_config) {
                Ok(dto) => HttpResponse::Ok().json(dto),
                Err(err) => {
                    error!("Cannot convert the MatcherConfig into a DTO. Err: {}", err);
                    HttpResponse::InternalServerError().finish()
                }
            }
        })
    }

    // ToDo: to be implemented in TOR-89
    pub fn send_event(&self, _req: HttpRequest) -> impl Future<Item = HttpResponse, Error = AWError> {
        debug!("API - received send_event request");
        let event = Event::new("fake_event");

        self.api_handler.send_event(event).map_err(AWError::from).and_then(|processed_event| {
            println!("Processed event: \n{:?}", processed_event);
            HttpResponse::Ok().finish()
        })

    }
}
