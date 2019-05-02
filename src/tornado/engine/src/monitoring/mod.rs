use actix_web::http::Method;
use actix_web::{HttpRequest, HttpResponse, Json, Result, Scope};
use chrono::prelude::Local;
use serde_derive::{Deserialize, Serialize};

pub fn monitoring_app(scope: Scope<()>) -> Scope<()> {
    scope
        .resource("", |r| r.method(Method::GET).f(index))
        .resource("/ping", |r| r.method(Method::GET).f(pong))
}

fn index(_req: &HttpRequest) -> HttpResponse {
    HttpResponse::Ok().content_type("text/html").body(
        r##"
        <div>
            <h1>Available endpoints:</h1>
            <ul>
                <li><a href="/monitoring/ping">Ping</a></li>
            </ul>
        </div>
        "##,
    )
}

#[derive(Serialize, Deserialize)]
pub struct PongResponse {
    pub message: String,
}

fn pong(_req: &HttpRequest) -> Result<Json<PongResponse>> {
    let dt = Local::now(); // e.g. `2014-11-28T21:45:59.324310806+09:00`
    let created_ms: String = dt.to_rfc3339();
    Ok(Json(PongResponse { message: format!("pong - {}", created_ms) }))
}

#[cfg(test)]
mod test {
    use super::*;
    use actix_web::client::ClientResponse;
    use actix_web::test::TestServer;
    use actix_web::{http, App, HttpMessage};
    use chrono::DateTime;
    use serde::de::DeserializeOwned;

    #[test]
    fn index_should_have_links_to_the_endpoints() {
        // Arrange
        let mut srv = TestServer::with_factory(|| {
            App::new().scope("/monitoring", |scope| monitoring_app(scope))
        });

        // Act
        let request = srv.client(http::Method::GET, "/monitoring").finish().unwrap();
        let response: ClientResponse = srv.execute(request.send()).unwrap();

        // Assert
        assert!(response.status().is_success());

        let body = body_to_string(&mut srv, &response);
        assert!(body.contains(r#"<a href="/monitoring/ping">"#));
    }

    #[test]
    fn ping_should_return_pong() {
        // Arrange
        let mut srv = TestServer::with_factory(|| {
            App::new().scope("/monitoring", |scope| monitoring_app(scope))
        });

        // Act
        let request = srv.client(http::Method::GET, "/monitoring/ping").finish().unwrap();
        let response: ClientResponse = srv.execute(request.send()).unwrap();

        // Assert
        assert!(response.status().is_success());

        let pong: PongResponse = body_to_json(&mut srv, &response).unwrap();
        assert!(pong.message.contains("pong - "));

        let date = DateTime::parse_from_rfc3339(&pong.message.clone()[7..]);
        // Assert
        assert!(date.is_ok());
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
