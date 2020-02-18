use tornado_common_api::Event;
use tornado_common::actors::nats_streaming_publisher::NatsPublisherActor;
use rants::Client;
use futures::stream::StreamExt;
use tornado_common::actors::message::EventMessage;

const BASE_ADDRESS: &str = "127.0.0.1:4222";

#[actix_rt::test]
async fn should_publish_to_nats_streaming() {
    let subject = "test_subject";

    let subscriber = Client::new(vec![BASE_ADDRESS.parse().unwrap()]);
    subscriber.connect().await;
    let (_, mut subscription) = subscriber.subscribe(&subject.parse().unwrap(), 1024).await.unwrap();

    let publisher = NatsPublisherActor::start_new(BASE_ADDRESS, subject, 10).await;
    publisher.do_send(EventMessage { event: Event::new("an_event") });

    // Read a message from the subscription
    let message = subscription.next().await.unwrap();
    let message = String::from_utf8(message.into_payload()).unwrap();
    println!("Received '{}'", message);
}
