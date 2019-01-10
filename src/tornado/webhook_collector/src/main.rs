use actix_web::http::Method;
use actix_web::{server, App, HttpRequest, Json, Responder, Result};
use serde_json::Value;
use crate::config::WebhookConfig;

mod config;

fn greet(req: &HttpRequest) -> impl Responder {
    let to = req.match_info().get("name").unwrap_or("World");
    println!("Saying hello to : {}", to);
    format!("Hello {}!\n", to)
}

fn post_event((_req, evt): (HttpRequest, Json<Value>)) -> Result<String> {

    Ok(format!("got event \n{:#?}\n", evt))
}



fn main() {
    let config = config::Conf::build();
    let config_dir = config.io.config_dir;
//    ciclo for che estrae i config file
    let config_file = "";


    server::new(|| create_app()).bind("0.0.0.0:80").expect("Can not bind to port 80").run();
}

fn parse_config_file(config_dir: &str) -> WebhookConfig {
    //FIXME: need to iterate over config_dir for config file
    config::build_config("").unwrap()
}

fn create_app() -> App {
    App::new()
        .resource("/", |r| r.method(Method::GET).f(greet))
        .resource("/event", |r| r.method(Method::POST).with(post_event))
}

#[cfg(test)]
mod test {

    use super::*;
    use actix_web::test::TestServer;
    use actix_web::{http, HttpMessage};
    use std::fs;

    #[test]
    fn should_get_hello_world() {
        // start new test server
        let mut srv = TestServer::with_factory(create_app);

        let request = srv.client(http::Method::GET, "/").finish().unwrap();
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
        let filename = "./test_resources/github-push-01.json";
        let github_json =
            fs::read_to_string(filename).expect(&format!("Unable to open the file [{}]", filename));
        let request = srv
            .client(http::Method::POST, "/event")
            .content_type("application/json")
            .body(github_json)
            .unwrap();
        let response = srv.execute(request.send()).unwrap();
        assert!(response.status().is_success());
    }

}
