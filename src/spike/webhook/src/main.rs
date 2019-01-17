use actix_web::http::Method;
use actix_web::{server, App, HttpRequest, Json, Responder, Result};
use crypto::hmac::Hmac;
use crypto::mac::Mac;
use crypto::sha1::Sha1;

fn greet(req: &HttpRequest) -> impl Responder {
    let to = req.match_info().get("name").unwrap_or("World");
    println!("Saying hello to : {}", to);
    format!("Hello {}!\n", to)
}

fn post_event((_req, evt): (HttpRequest, Json<jmespath::Variable>)) -> Result<String> {
    //let event = Event::new("webhook");
    let expr = jmespath::compile("commits[0].author.name").unwrap();
    let result = expr.search(evt.as_object()).unwrap();

    println!("!!!! {}\n ", result.as_string().unwrap());

    Ok(format!("got event \n{:#?}\n", evt))
}

fn post_event_signed((req, body): (HttpRequest, String)) -> Result<String> {
    validate_request(&req, &body)?;

    let evt: jmespath::Variable =
        serde_json::from_str(&body).map_err(actix_web::error::ErrorBadRequest)?;

    //let event = Event::new("webhook");
    let expr = jmespath::compile("commits[0].author.name").unwrap();
    let result = expr.search(evt.as_object()).unwrap();

    println!("!!!! {}\n ", result.as_string().unwrap());

    Ok(format!("got event \n{:#?}\n", evt))
}

fn validate_request(req: &HttpRequest, body: &str) -> Result<()> {
    let r = req.clone();
    let s: &str = r
        .headers()
        .get("X-Hub-Signature")
        .ok_or_else(|| actix_web::error::ErrorUnauthorized(actix_web::error::ParseError::Header))?
        .to_str()
        .map_err(actix_web::error::ErrorUnauthorized)?;

    // strip "sha1=" from the header
    let (_, sig) = s.split_at(5);

    println!("X-Hub-Signature header: \n{}", s);

    let secret = "github_secret_key";

    if is_valid_signature(&sig, &body, &secret) {
        Ok(())
    } else {
        Err(actix_web::error::ErrorUnauthorized(actix_web::error::ParseError::Header))
    }
}

fn is_valid_signature(signature: &str, body: &str, secret: &str) -> bool {
    let digest = Sha1::new();
    let mut hmac = Hmac::new(digest, secret.as_bytes());
    hmac.input(body.as_bytes());
    let expected_signature = hmac.result();
    // println!("Expected signature {:#?}", expected_signature.code().to_vec());
    crypto::util::fixed_time_eq(
        bytes_to_hex(&expected_signature.code().to_vec()).as_bytes(),
        signature.as_bytes(),
    )
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect::<Vec<String>>().join("")
}

fn main() {
    server::new(create_app).bind("0.0.0.0:80").expect("Can not bind to port 80").run();
}

fn create_app() -> App {
    App::new()
        .resource("/", |r| r.method(Method::GET).f(greet))
        .resource("/event", |r| r.method(Method::POST).with(post_event))
        .resource("/event_signed", |r| r.method(Method::POST).with(post_event_signed))
        .resource("/{name}", |r| r.method(Method::GET).f(greet))
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

    #[test]
    fn should_post_a_signed_event() {
        let mut srv = TestServer::with_factory(create_app);
        let filename = "./test_resources/github-push-02.json";
        let github_json =
            fs::read_to_string(filename).expect(&format!("Unable to open the file [{}]", filename));

        let request = srv
            .client(http::Method::POST, "/event_signed")
            .header("X-Hub-Signature", "sha1=89ac00d2c641c3136fa0c6dded600f935d8011bb")
            .content_type("application/json")
            .body(github_json)
            .unwrap();
        let response = srv.execute(request.send()).unwrap();
        assert!(response.status().is_success());
    }

    #[test]
    fn should_not_post_a_signed_event_without_proper_headers() {
        // start new test server
        let mut srv = TestServer::with_factory(create_app);
        let filename = "./test_resources/github-push-02.json";
        let github_json =
            fs::read_to_string(filename).expect(&format!("Unable to open the file [{}]", filename));
        let request = srv
            .client(http::Method::POST, "/event_signed")
            .header("X-Hub-Signature", "sha1=0123456789012345678901234567890123456789")
            .content_type("application/json")
            .body(github_json)
            .unwrap();
        let response = srv.execute(request.send()).unwrap();
        assert!(!response.status().is_success());
    }

}
