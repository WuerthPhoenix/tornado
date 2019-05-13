use crate::api::handler::ApiHandler;
use crate::convert::matcher_config_to_dto;
use actix_web::{HttpRequest, HttpResponse};
use std::sync::Arc;
use tornado_common_api::Event;

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

    pub fn get_config(&self, _req: HttpRequest) -> HttpResponse {

        // ToDo: remove "unwrap()". Could be investigated in TOR-89

        let matcher_config = self.api_handler.read().map_err(failure::Fail::compat).unwrap();
        HttpResponse::Ok().json(matcher_config_to_dto(matcher_config).unwrap())
    }

    // ToDo: to be implemented in TOR-89
    pub fn test(&self, _req: HttpRequest) -> HttpResponse {

        let event = Event::new("fake_event");
        let processed_event = self.api_handler.send_event(event).map_err(failure::Fail::compat).unwrap();
        println!("Processed event: \n{:?}", processed_event);
        HttpResponse::Ok().finish()
    }
}
