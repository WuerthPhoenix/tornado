use actix::prelude::*;
use serial_test::serial;
use std::sync::Arc;
use std::sync::Mutex;
use tornado_common::actors::json_event_reader::JsonEventReaderActor;
use tornado_common::actors::message::EventMessage;
use tornado_common::actors::tcp_client::TcpClientActor;
use tornado_common::actors::tcp_server::listen_to_tcp;
use tornado_common_api::Event;

const BASE_ADDRESS: &str = "127.0.0.1";

#[test]
#[serial]
fn should_perform_a_tcp_request() {
    let received = Arc::new(Mutex::new(None));

    let act_received = received.clone();
    System::run(move || {
        let port = port_check::free_local_port().unwrap();
        let address = format!("{}:{}", BASE_ADDRESS, port);

        println!("Creating server at: {}", address);
        let tcp_create = listen_to_tcp(address.clone(), 10000, move |msg| {
            println!("Received a connection request");
            let json_act_received = act_received.clone();
            JsonEventReaderActor::start_new(msg, 10000, move |event| {
                println!("JsonEventReaderActor -  received an event");
                let mut lock = json_act_received.lock().unwrap();
                *lock = Some(event);
                System::current().stop();
            });
        });

        actix::spawn(async move {
            tcp_create.await.unwrap();

            let client_addr = TcpClientActor::start_new(address.clone(), 16);
            client_addr.do_send(EventMessage { event: Event::new("an_event") });
        });
    })
    .unwrap();

    let event = received.lock().unwrap();
    assert!(event.is_some());
    assert_eq!("an_event", event.as_ref().unwrap().event_type);
}
