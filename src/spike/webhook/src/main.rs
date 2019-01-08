use actix_web::http::Method;
use actix_web::{server, App, HttpRequest, Json, Responder, Result};
use tornado_common_api::Event;

fn greet(req: &HttpRequest) -> impl Responder {
    let to = req.match_info().get("name").unwrap_or("World");
    println!("Saying hello to : {}", to);
    format!("Hello {}!\n", to)
}

fn get_event(evt: Json<Event>) -> Result<String> {
    println!("received event: \n{:#?}", evt);
    Ok(format!("got event \n{:#?}\n", evt))
}

fn main() {
    server::new(|| {
        App::new()
            .resource("/", |r| r.f(greet))
            .resource("/{name}", |r| r.f(greet))
            .resource("/event", |r| r.method(Method::POST).with(get_event))
    })
    .bind("0.0.0.0:80")
    .expect("Can not bind to port 80")
    .run();
}
