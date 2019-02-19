use actix::Actor;
use actix_web::client::{ClientConnector, ClientRequest};
//use actix_web::http::Method;
//use actix_web::{client, App, HttpMessage, HttpRequest, Json, Responder, Result};
use futures::future::Future;
use http::header;
use openssl::ssl::{SslConnector, SslMethod, SslVerifyMode};
use serde_derive::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::time::Duration;
use actix_web::server::HttpServer;

fn main() {
    // this curl command works as expected:
    // curl -k -H 'Accept: application/json' -X POST 'https://root:dd43ee24bfb9630f@127.0.0.1:5665/v1/events?queue=america&types=CheckResult'
    let url = "https://127.0.0.1:5665/v1/events";
    let user = "root";
    let pass = "dd43ee24bfb9630f";
    //run_actix(url, user, pass);
    run_reqwest(url, user, pass);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Filter {
    types: Vec<EventType>,
    queue: String,
    filter: Option<String>,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum EventType {
    CheckResult,
    StateChange,
    Notification,
    AcknowledgementSet,
    AcknowledgementCleared,
    CommentAdded,
    CommentRemoved,
    DowntimeAdded,
    DowntimeRemoved,
    DowntimeStarted,
    DowntimeTriggered,
}

fn run_reqwest(url: &str, user: &str, pass: &str) {
    let request_body = Filter {
        types: vec![EventType::CheckResult],
        queue: "test-queue".to_owned(),
        filter: None,
    };

    let client = reqwest::ClientBuilder::new()
        .danger_accept_invalid_certs(true)
        .timeout(None)
        .build()
        .unwrap();

    //let client = reqwest::Client::new();

    println!("Prepare request");

    let response = client
        .post(url)
        .header(header::ACCEPT, "application/json")
        .basic_auth(user, Some(pass))
        .json(&request_body)
        .send()
        .unwrap();

    println!("Got a response");

    let mut reader = BufReader::new(response);

    let mut line = String::new();
    let _len = reader.read_line(&mut line).unwrap();
    println!("First line is [{}]", &line);

    /*
    let mut buf: Vec<u8> = vec![];
    response.copy_to(&mut buf).unwrap();
    println!("buf is [{:#?}]", buf.as_slice());
    */
}

fn run_actix(url: &str, user: &str, pass: &str) {
    actix::run(|| {
        let mut ssl_conn_builder = SslConnector::builder(SslMethod::tls()).unwrap();
        ssl_conn_builder.set_verify(SslVerifyMode::NONE);
        let ssl_conn = ssl_conn_builder.build();
        let connector = ClientConnector::with_connector(ssl_conn).start();

        let request_body = Filter {
            types: vec![EventType::StateChange],
            queue: "test-queue".to_owned(),
            filter: None,
        };

        let auth = format!("{}:{}", user, pass);
        let header_value = format!("Basic {}", base64::encode(&auth));

        ClientRequest::post(url)
            .with_connector(connector)
            .header(header::ACCEPT, "application/json")
            .header(header::AUTHORIZATION, header_value)
            .timeout(Duration::from_secs(999_999))
            .json(request_body)
            .unwrap()
            .send()
            .map_err(|err| panic!("Connection failed. Err: {}", err))
            .and_then(|response| {
                println!("Response: {:?}", response);
                /*
                                response.body().map_err(|_| ()).map(|bytes| {
                                    println!("Body");
                                    println!("{:?}", bytes);
                                    ()
                                });
                */
                Ok(())
            })
    });
}
