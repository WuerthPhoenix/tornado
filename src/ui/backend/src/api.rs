use crate::convert::matcher_config_to_dto;
use actix_web::http::Method;
use actix_web::{App, HttpRequest, Json, Result};
use std::sync::Arc;
use tornado_engine_matcher::config::MatcherConfig;
use tornado_engine_matcher::error::MatcherError;

pub mod matcher;

pub fn new_app<T: ApiHandler + 'static>(api_handler: Arc<T>) -> App {
    let http = HttpHandler { api_handler };

    let mut app = App::new();

    let http_clone = http.clone();
    app = app.resource("/api/config", |resource| {
        resource.method(Method::GET).f(move |req| http_clone.get_config(req))
    });

    app
}

pub trait ApiHandler {
    fn read(&self) -> Result<MatcherConfig, MatcherError>;
}

struct HttpHandler<T: ApiHandler> {
    api_handler: Arc<T>,
}

impl<T: ApiHandler> Clone for HttpHandler<T> {
    fn clone(&self) -> Self {
        HttpHandler { api_handler: self.api_handler.clone() }
    }
}

impl<T: ApiHandler> HttpHandler<T> {
    fn get_config(&self, _req: &HttpRequest) -> Result<Json<dto::config::MatcherConfigDto>> {
        let matcher_config = self.api_handler.read().map_err(failure::Fail::compat)?;

        Ok(Json(matcher_config_to_dto(matcher_config)?))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use actix_web::client::ClientResponse;
    use actix_web::test::TestServer;
    use actix_web::{http, HttpMessage};
    use serde::de::DeserializeOwned;

    struct TestApiHandler {}

    impl ApiHandler for TestApiHandler {
        fn read(&self) -> Result<MatcherConfig, MatcherError> {
            Ok(MatcherConfig::Rules { rules: vec![] })
        }
    }

    #[test]
    fn should_return_the_matcher_config() {
        // Arrange
        let mut srv = TestServer::with_factory(|| new_app(Arc::new(TestApiHandler {})));

        // Act
        let request = srv.client(http::Method::GET, "/api/config").finish().unwrap();
        let response: ClientResponse = srv.execute(request.send()).unwrap();

        // Assert
        assert!(response.status().is_success());

        let dto: dto::config::MatcherConfigDto = body_to_json(&mut srv, &response).unwrap();
        assert_eq!(dto::config::MatcherConfigDto::Rules { rules: vec![] }, dto);
    }

    fn body_to_string(srv: &mut TestServer, response: &ClientResponse) -> String {
        let bytes = srv.execute(response.body()).unwrap();
        std::str::from_utf8(&bytes).unwrap().to_owned()
    }

    fn body_to_json<T: DeserializeOwned>(
        srv: &mut TestServer,
        response: &ClientResponse,
    ) -> serde_json::error::Result<T> {
        serde_json::from_str(&body_to_string(srv, response))
    }
}
