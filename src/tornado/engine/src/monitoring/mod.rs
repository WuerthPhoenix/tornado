use actix_web::web::Json;
use actix_web::{web, HttpRequest, HttpResponse, Result, Scope};
use chrono::prelude::Local;
use serde_derive::{Deserialize, Serialize};

pub fn monitoring_endpoints(scope: Scope) -> Scope {
    scope
        .service(web::resource("").route(web::get().to(index)))
        .service(web::resource("/ping").route(web::get().to(pong)))
    /*
    scope
        .resource("", |r| r.method(Method::GET).f(index))
        .resource("/ping", |r| r.method(Method::GET).f(pong))
        */
}

fn index(_req: HttpRequest) -> HttpResponse {
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

fn pong(_req: HttpRequest) -> Result<Json<PongResponse>> {
    let dt = Local::now(); // e.g. `2014-11-28T21:45:59.324310806+09:00`
    let created_ms: String = dt.to_rfc3339();
    Ok(Json(PongResponse { message: format!("pong - {}", created_ms) }))
}

#[cfg(test)]
mod test {
    use super::*;
    use actix_web::{test, App};
    use chrono::DateTime;

    #[test]
    fn index_should_have_links_to_the_endpoints() {
        // Arrange
        let mut srv =
            test::init_service(App::new().service(monitoring_endpoints(web::scope("/monitoring"))));

        // Act
        let request = test::TestRequest::get().uri("/monitoring").to_request();
        let response = test::read_response(&mut srv, request);

        // Assert
        let body = std::str::from_utf8(&response).unwrap();
        assert!(body.contains(r#"<a href="/monitoring/ping">"#));
    }

    #[test]
    fn ping_should_return_pong() {
        // Arrange
        let mut srv =
            test::init_service(App::new().service(monitoring_endpoints(web::scope("/monitoring"))));

        // Act
        let request = test::TestRequest::get().uri("/monitoring/ping").to_request();

        // Assert
        let pong: PongResponse = test::read_response_json(&mut srv, request);
        assert!(pong.message.contains("pong - "));

        let date = DateTime::parse_from_rfc3339(&pong.message.clone()[7..]);
        // Assert
        assert!(date.is_ok());
    }

}
