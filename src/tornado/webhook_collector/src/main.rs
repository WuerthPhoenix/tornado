use actix_web::http::Method;
use actix_web::{server, App, HttpRequest, Json, Responder, Result};
use serde_json::Value;
use tornado_common_logger::setup_logger;

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

    setup_logger(&config.logger).expect("Cannot configure the logger");

    let webhooks_dir = format!("{}/{}", config.io.config_dir, config.io.webhooks_dir);
    let webhooks_config = config::read_webhooks_from_config(&webhooks_dir)
        .expect("Cannot parse the webhooks configuration");

    server::new(|| create_app()).bind("0.0.0.0:80").expect("Can not bind to port 80").run();
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

}
