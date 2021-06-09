use tornado_common::actors::json_event_reader::JsonEventReaderActor;
use tornado_common::actors::message::EventMessage;
use tornado_common::actors::tcp_client::TcpClientActor;
use tornado_common::actors::tcp_server::listen_to_tcp;
use tornado_common_api::Event;

const BASE_ADDRESS: &str = "127.0.0.1";

#[actix_rt::test]
async fn should_perform_a_tcp_request() {
    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();

    let port = port_check::free_local_port().unwrap();
    let address = format!("{}:{}", BASE_ADDRESS, port);

    println!("Creating server at: {}", address);
    let tcp_create = listen_to_tcp(address.clone(), 10000, move |msg| {
        println!("Received a connection request");
        let sender = sender.clone();
        JsonEventReaderActor::start_new(msg, 10000, move |event| {
            println!("JsonEventReaderActor -  received an event");
            sender.send(event).unwrap();
        });
    });

    actix::spawn(async move {
        tcp_create.await.unwrap();

        let client_addr = TcpClientActor::start_new(address.clone(), 16);
        client_addr.do_send(EventMessage { event: Event::new("an_event") });
    });

    let event = receiver.recv().await.unwrap();
    assert_eq!("an_event", event.event_type);
}
