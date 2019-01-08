use actix_web::http::Method;
use actix_web::{server, App, HttpRequest, Json, Responder, Result};
use serde_json::Value;
use tornado_common_api::Event;

fn greet(req: &HttpRequest) -> impl Responder {
    let to = req.match_info().get("name").unwrap_or("World");
    println!("Saying hello to : {}", to);
    format!("Hello {}!\n", to)
}

fn post_event(evt: Json<jmespath::Variable>) -> Result<String> {
    let mut event = Event::new("webhook");
    // Act
    let expr = jmespath::compile("commits[0].author.name").unwrap();
//    let data = jmespath::Variable::from_json(&event_json).unwrap();

    let result = expr.search(evt.as_object()).unwrap();

    // Assert

    println!("!!!! {}\n ", result.as_string().unwrap());
//    assert_eq!("email", result.as_string().unwrap());
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
    use std::fs;

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
        let filename = "./test_resources/github-push.json";
        let github_json = fs::read_to_string(filename).expect(&format!("Unable to open the file [{}]", filename));
        let request = srv.client(
            http::Method::POST, "/event").content_type("application/json").body(github_json).unwrap();
        let response = srv.execute(request.send()).unwrap();
        assert!(response.status().is_success());

        /*
        let bytes = srv.execute(response.body()).unwrap();
        let body = str::from_utf8(&bytes).unwrap();
        assert_eq!(body, "Hello world!");
        */
    }

//    GitHub Headers:
//    Request URL: http://35.240.75.114/event
//    Request method: POST
//    content-type: application/json
//    Expect:
//    User-Agent: GitHub-Hookshot/60c6631
//    X-GitHub-Delivery: 78aca354-1335-11e9-8f50-3fb6173890ad
//    X-GitHub-Event: push



}