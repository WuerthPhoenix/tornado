use actix::prelude::*;
use futures::Stream;
use std::fs;
use tokio_uds::*;

pub fn listen_to_uds_socket<
    P: Into<String>,
    F: 'static + FnMut(UdsConnectMessage) -> () + Sized,
>(
    path: P,
    callback: F,
) {
    let path_string = path.into();
    let listener = match UnixListener::bind(&path_string) {
        Ok(m) => m,
        Err(_) => {
            fs::remove_file(&path_string).unwrap();
            UnixListener::bind(&path_string).unwrap()
        }
    };

    UdsServerActor::create(|ctx| {
        ctx.add_message_stream(listener.incoming().map_err(|e| panic!("err={:?}", e)).map(
            |stream| {
                //let addr = stream.peer_addr().unwrap();
                UdsConnectMessage{stream}
            },
        ));
        UdsServerActor { path: path_string, callback }
    });
}

struct UdsServerActor<F>
where
    F: 'static + FnMut(UdsConnectMessage) -> () + Sized,
{
    path: String,
    callback: F,
}

impl<F> Actor for UdsServerActor<F>
where
    F: 'static + FnMut(UdsConnectMessage) -> () + Sized,
{
    type Context = Context<Self>;
}

#[derive(Message)]
pub struct UdsConnectMessage{
    pub stream: UnixStream
}

/// Handle stream of UnixStream's
impl<F> Handler<UdsConnectMessage> for UdsServerActor<F>
where
    F: 'static + FnMut(UdsConnectMessage) -> () + Sized,
{
    type Result = ();

    fn handle(&mut self, msg: UdsConnectMessage, _: &mut Context<Self>) {
        info!("UdsServerActor - new client connected to [{}]", &self.path);
        (&mut self.callback)(msg);
    }
}
