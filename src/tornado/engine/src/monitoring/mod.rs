use actix_web::http::Method;
use actix_web::{App, HttpRequest, Responder};
use chrono::prelude::Local;

fn pong(_req: &HttpRequest) -> impl Responder {
    let dt = Local::now(); // e.g. `2014-11-28T21:45:59.324310806+09:00`
    let created_ms: String = dt.to_rfc3339();
    format!("pong - {}", created_ms)
}

pub fn monitoring_app() -> App {
    App::new().resource("/monitoring/ping", |r| r.method(Method::GET).f(pong))
}

#[cfg(test)]
mod test {
    use super::*;
    use actix_web::test::TestServer;
    use actix_web::{http, HttpMessage};

    #[test]
    fn ping_should_return_pong() {
        // Arrange
        let mut srv = TestServer::with_factory(|| monitoring_app());

        // Act
        let request = srv.client(http::Method::GET, "/monitoring/ping").finish().unwrap();
        let response = srv.execute(request.send()).unwrap();

        // Assert
        assert!(response.status().is_success());

        let bytes = srv.execute(response.body()).unwrap();
        let body = std::str::from_utf8(&bytes).unwrap();

        assert!(body.contains("pong - "));
    }
}
