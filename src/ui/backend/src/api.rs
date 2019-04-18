use actix_web::http::Method;
use actix_web::{App, HttpRequest, Json, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tornado_engine_matcher::config::MatcherConfig;
use tornado_engine_matcher::error::MatcherError;

pub mod matcher;

pub fn new_app<T: ApiHandler + 'static>(api_handler: T) -> App {
    let http = Arc::new(HttpHandler { api_handler });

    let mut app = App::new();

    let http_clone = http.clone();
    app = app.resource("/api/ping", |resource| {
        resource.method(Method::GET).f(move |req| http_clone.pong(req))
    });

    let http_clone = http.clone();
    app = app.resource("/api/config", |resource| {
        resource.method(Method::GET).f(move |req| http_clone.get_config(req))
    });

    app
}

pub trait ApiHandler {
    fn pong(&self) -> PongResponse {
        PongResponse { message: format!("pong") }
    }

    fn read(&self) -> Result<MatcherConfig, MatcherError>;

}

struct HttpHandler<T: ApiHandler> {
    api_handler: T,
}

impl<T: ApiHandler> HttpHandler<T> {
    fn pong(&self, _req: &HttpRequest) -> Result<Json<PongResponse>> {
        Ok(Json(self.api_handler.pong()))
    }

    fn get_config(&self, _req: &HttpRequest) -> Result<Json<dto::config::MatcherConfig>> {
        Ok(Json(dto::config::MatcherConfig::Rules {rules: vec![]}))
    }

}

#[derive(Serialize, Deserialize)]
pub struct PongResponse {
    pub message: String,
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
            Ok(MatcherConfig::Rules {rules: vec![]})
        }
    }

    #[test]
    fn ping_should_return_pong() {
        // Arrange
        let mut srv = TestServer::with_factory(|| new_app(TestApiHandler {}));

        // Act
        let request = srv.client(http::Method::GET, "/api/ping").finish().unwrap();
        let response: ClientResponse = srv.execute(request.send()).unwrap();

        // Assert
        assert!(response.status().is_success());

        let pong: PongResponse = body_to_json(&mut srv, &response).unwrap();
        assert!(pong.message.eq("pong"));
    }

    #[test]
    fn should_return_the_matcher_config() {
        // Arrange
        let mut srv = TestServer::with_factory(|| new_app(TestApiHandler {}));

        // Act
        let request = srv.client(http::Method::GET, "/api/config").finish().unwrap();
        let response: ClientResponse = srv.execute(request.send()).unwrap();

        // Assert
        assert!(response.status().is_success());

        let dto: dto::config::MatcherConfig = body_to_json(&mut srv, &response).unwrap();
        assert_eq!(dto::config::MatcherConfig::Rules {rules: vec![]}, dto);
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
