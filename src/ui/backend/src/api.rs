use actix_web::http::Method;
use actix_web::{App, HttpRequest, Json, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub mod matcher;

pub fn new_app<T: ApiHandler + 'static>(api_handler: T) -> App {
    let http = Arc::new(HttpHandler { api_handler });

    let mut app = App::new();

    let http_clone = http.clone();
    app = app.resource("/monitoring/ping", |resource| {
        resource.method(Method::GET).f(move |req| http_clone.pong(req))
    });

    let http_clone = http.clone();
    app = app.resource("/monitoring/ping2", |resource| {
        resource.method(Method::GET).f(move |req| http_clone.pong(req))
    });

    app
}

pub trait ApiHandler {
    fn pong(&self) -> PongResponse {
        PongResponse { message: format!("pong") }
    }
}

struct HttpHandler<T: ApiHandler> {
    api_handler: T,
}

impl<T: ApiHandler> HttpHandler<T> {
    fn pong(&self, _req: &HttpRequest) -> Result<Json<PongResponse>> {
        Ok(Json(self.api_handler.pong()))
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
    impl ApiHandler for TestApiHandler {}

    #[test]
    fn ping_should_return_pong() {
        // Arrange
        let mut srv = TestServer::with_factory(|| new_app(TestApiHandler {}));

        // Act
        let request = srv.client(http::Method::GET, "/monitoring/ping").finish().unwrap();
        let response: ClientResponse = srv.execute(request.send()).unwrap();

        // Assert
        assert!(response.status().is_success());

        let pong: PongResponse = body_to_json(&mut srv, &response).unwrap();
        assert!(pong.message.eq("pong"));
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
