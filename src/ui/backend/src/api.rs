use self::handler::ApiHandler;
use self::http::HttpHandler;
use actix_web::http::Method;
use actix_web::Scope;
use std::sync::Arc;

mod handler;
mod http;
pub mod matcher;

pub fn new_app<T: ApiHandler + 'static>(mut scope: Scope<()>, api_handler: Arc<T>) -> Scope<()> {
    let http = HttpHandler { api_handler };

    let http_clone = http.clone();

    scope = scope.resource("/config", |resource| {
        resource.method(Method::GET).f(move |req| http_clone.get_config(req))
    });

    scope
}

#[cfg(test)]
mod test {
    use super::*;
    use actix_web::client::ClientResponse;
    use actix_web::test::TestServer;
    use actix_web::{http, App, HttpMessage};
    use serde::de::DeserializeOwned;
    use tornado_engine_matcher::config::MatcherConfig;
    use tornado_engine_matcher::error::MatcherError;

    struct TestApiHandler {}

    impl ApiHandler for TestApiHandler {
        fn read(&self) -> Result<MatcherConfig, MatcherError> {
            Ok(MatcherConfig::Rules { rules: vec![] })
        }
    }

    #[test]
    fn should_return_the_matcher_config() {
        // Arrange
        let mut srv = TestServer::with_factory(|| {
            App::new().scope("/api", |scope| new_app(scope, Arc::new(TestApiHandler {})))
        });

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
