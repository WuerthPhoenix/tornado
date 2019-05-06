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

#[cfg(test)]
mod test {
    use super::*;
    use actix_service::Service;
    use actix_web::{http::{StatusCode}, App, test};
    use tornado_engine_matcher::config::MatcherConfig;
    use tornado_engine_matcher::error::MatcherError;

    struct TestApiHandler {}

    impl ApiHandler for TestApiHandler {
        fn read(&self) -> Result<MatcherConfig, MatcherError> {
            Ok(MatcherConfig::Rules { rules: vec![] })
        }
    }

    #[test]
    fn should_return_status_code_ok() {
        // Arrange
        let mut srv = test::init_service(
            App::new().service(
                new_app(web::scope("/api"), Arc::new(TestApiHandler {}))
            )
        );

        // Act
        let request = test::TestRequest::get()
            .uri("/api/config")
            //.header(header::CONTENT_TYPE, "application/json")
            //.set_payload(payload)
            .to_request();

        let response = test::block_on(srv.call(request)).unwrap();

        // Assert
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn should_return_the_matcher_config() {
        // Arrange
        let mut srv = test::init_service(
            App::new().service(
                new_app(web::scope("/api"), Arc::new(TestApiHandler {}))
            )
        );

        // Act
        let request = test::TestRequest::get()
            .uri("/api/config")
            //.header(header::CONTENT_TYPE, "application/json")
            //.set_payload(payload)
            .to_request();

        // Assert
        let dto: dto::config::MatcherConfigDto = test::read_response_json(&mut srv, request);
        assert_eq!(dto::config::MatcherConfigDto::Rules { rules: vec![] }, dto);
    }

}
