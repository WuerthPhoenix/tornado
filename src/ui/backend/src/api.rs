use self::handler::ApiHandler;
use self::http::HttpHandler;
use actix_web::{web, Scope};
use std::sync::Arc;

mod handler;
mod http;
pub mod matcher;

pub fn new_app<T: ApiHandler + 'static>(mut scope: Scope, api_handler: Arc<T>) -> Scope {
    let http = HttpHandler { api_handler };

    let http_clone = http.clone();

    scope = scope.service(
        web::resource("/config").route(
            web::get()
                .to(move |req| http_clone.get_config(req))
        ),
    );

    scope
}
/*
#[cfg(test)]
mod test {
    use super::*;
    use actix_http_test::{TestServer, TestServerRuntime};
    use actix_web::client::ClientResponse;
    use actix_web::{http, App};
    use serde::de::DeserializeOwned;
    use tornado_engine_matcher::config::MatcherConfig;
    use tornado_engine_matcher::error::MatcherError;
    use actix_http::HttpService;

    struct TestApiHandler {}

    impl ApiHandler for TestApiHandler {
        fn read(&self) -> Result<MatcherConfig, MatcherError> {
            Ok(MatcherConfig::Rules { rules: vec![] })
        }
    }

    #[test]
    fn should_return_the_matcher_config() {
        // Arrange
        let mut srv = TestServer::new(|| {
            HttpService::new(App::new().service(
                new_app(web::scope("/api"), Arc::new(TestApiHandler {}))
            ))
        });

        // Act
        let request = srv.get("/api/config");
        let mut response = srv.block_on(request.send()).unwrap();

        // Assert
        assert!(response.status().is_success());

        let dto: dto::config::MatcherConfigDto = body_to_json(&mut srv, &mut response).unwrap();
        assert_eq!(dto::config::MatcherConfigDto::Rules { rules: vec![] }, dto);
    }

    fn body_to_string(srv: &mut TestServerRuntime, response: &mut ClientResponse) -> String {
        let bytes = &srv.execute(response.body()).unwrap();
        std::str::from_utf8(bytes).unwrap().to_owned()
    }

    fn body_to_json<T: DeserializeOwned>(
        srv: &mut TestServerRuntime,
        response: &mut ClientResponse,
    ) -> serde_json::error::Result<T> {
        serde_json::from_str(&body_to_string(srv, response))
    }
}
*/