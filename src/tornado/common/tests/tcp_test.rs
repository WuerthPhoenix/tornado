use actix::prelude::*;
use std::net::TcpListener;
use std::sync::Arc;
use std::sync::Mutex;
use tornado_common::actors::json_event_reader::JsonEventReaderActor;
use tornado_common::actors::tcp_client::{EventMessage, TcpClientActor};
use tornado_common::actors::tcp_server::listen_to_tcp;
use tornado_common_api::Event;

const BASE_ADDRESS: &str = "127.0.0.1";

#[test]
fn should_perform_a_tcp_request() {
    let received = Arc::new(Mutex::new(None));

    let act_received = received.clone();
    System::run(move || {
        let port = get_available_port().unwrap();
        let address = format!("{}:{}", BASE_ADDRESS, port);

        println!("Creating server at: {}", address);
        listen_to_tcp(address.clone(), move |msg| {
            println!("Received a connection request");
            let json_act_received = act_received.clone();
            JsonEventReaderActor::start_new(msg, move |event| {
                println!("JsonEventReaderActor -  received an event");
                let mut lock = json_act_received.lock().unwrap();
                *lock = Some(event);
                System::current().stop();
            });
        })
        .unwrap();

        let client_addr = TcpClientActor::start_new(address.clone(), 16);
        client_addr.do_send(EventMessage { event: Event::new("an_event") });
    });

    let event = received.lock().unwrap();
    assert!(event.is_some());
    assert_eq!("an_event", event.as_ref().unwrap().event_type);
}

fn port_is_available(port: u16) -> bool {
    match TcpListener::bind((BASE_ADDRESS, port)) {
        Ok(_) => true,
        Err(_) => false,
    }
}

fn get_available_port() -> Option<u16> {
    (10000..65535).find(|port| port_is_available(*port))
}
