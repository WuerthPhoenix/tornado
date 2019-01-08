use actix_web::http::Method;
use actix_web::{server, App, HttpRequest, Json, Responder, Result};
use serde_json::Value;

fn greet(req: &HttpRequest) -> impl Responder {
    let to = req.match_info().get("name").unwrap_or("World");
    println!("Saying hello to : {}", to);
    format!("Hello {}!\n", to)
}

fn post_event(evt: Json<Value>) -> Result<String> {
    println!("received event: \n{:#?}", evt);
    Ok(format!("got event \n{:#?}\n", evt))
}

fn main() {
    server::new(|| {
        create_app()
    })
    .bind("0.0.0.0:80")
    .expect("Can not bind to port 80")
    .run();
}

fn create_app() -> App {
    App::new()
        .resource("/", |r| r.method(Method::GET).f(greet))
        .resource("/event", |r| r.method(Method::POST).with(post_event))
        .resource("/{name}", |r| r.method(Method::GET).f(greet))
}

#[cfg(test)]
mod test {

    use super::*;
    use actix_web::{HttpRequest, HttpMessage, http};
    use actix_web::test::TestServer;
    use tornado_common_api::Event;

    #[test]
    fn should_get_hello_world() {
        // start new test server
        let mut srv = TestServer::with_factory(create_app);

        let request = srv.client(
            http::Method::GET, "/").finish().unwrap();
        let response = srv.execute(request.send()).unwrap();
        assert!(response.status().is_success());

        let bytes = srv.execute(response.body()).unwrap();
        let body = std::str::from_utf8(&bytes).unwrap();
        assert_eq!(body, "Hello World!\n");
    }

    #[test]
    fn should_post_an_event() {
        // start new test server
        let mut srv = TestServer::with_factory(create_app);

        let request = srv.client(
            http::Method::POST, "/event").json(Event::new("Prova1")).unwrap();
        let response = srv.execute(request.send()).unwrap();
        assert!(response.status().is_success());

        /*
        let bytes = srv.execute(response.body()).unwrap();
        let body = str::from_utf8(&bytes).unwrap();
        assert_eq!(body, "Hello world!");
        */
    }
}