use tornado_common_api::Event;
use tornado_common::actors::nats_streaming_publisher::NatsPublisherActor;
use rants::Client;
use futures::stream::StreamExt;
use tornado_common::actors::message::EventMessage;
use tornado_common::actors::nats_streaming_subscriber::subscribe_to_nats_streaming;
use std::sync::{Arc, Mutex};
use tokio::time;

const BASE_ADDRESS: &str = "127.0.0.1:4222";

#[actix_rt::test]
async fn should_publish_to_nats_streaming() {
    let random =
    let subject = "test_subject";

    let received = Arc::new(Mutex::new(None));

    let received_clone = received.clone();
    subscribe_to_nats_streaming(BASE_ADDRESS, subject, move |event| {
        let mut lock = received_clone.lock().unwrap();
        *lock = Some(event.event);
        Ok(())
    }).await.unwrap();

    let publisher = NatsPublisherActor::start_new(BASE_ADDRESS, subject, 10).await;
    publisher.do_send(EventMessage { event: Event::new("an_event") });

    time::delay_until(time::Instant::now() + time::Duration::new(2, 0)).await;

    let event = received.lock().unwrap();
    assert!(event.is_some());
    assert_eq!("an_event", event.as_ref().unwrap().event_type);

}
