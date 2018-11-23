use actix::prelude::*;
use serde_json;
use std::io::Write;
use std::os::unix::net::UnixStream;
use tornado_common_api;

pub struct EventMessage {
    pub event: tornado_common_api::Event,
}

impl Message for EventMessage {
    type Result = Result<(), serde_json::Error>;
}

pub struct UdsWriterActor {
    pub stream: UnixStream,
}

impl Actor for UdsWriterActor {
    type Context = SyncContext<Self>;
}

impl Handler<EventMessage> for UdsWriterActor {
    type Result = Result<(), serde_json::Error>;

    fn handle(&mut self, msg: EventMessage, _: &mut SyncContext<Self>) -> Self::Result {
        trace!("UdsWriterActor - {:?} - received new event", &msg.event);
        let event_bytes = serde_json::to_vec(&msg.event).unwrap();
        self.stream.write_all(&event_bytes).expect("should write event to socket");
        self.stream.write_all(b"\n").expect("should write endline to socket");
        Ok(())
    }
}
