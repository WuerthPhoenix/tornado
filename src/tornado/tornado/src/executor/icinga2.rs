use actix::prelude::*;
use actix_web::client::{ClientConnector, ClientRequest};
use failure_derive::Fail;
use futures::future::Future;
use http::header;
use log::*;
use std::time::Duration;

pub struct Icinga2ApiClientMessage {
}

impl Message for Icinga2ApiClientMessage {
    type Result = Result<(), Icinga2ApiClientActorError>;
}

#[derive(Fail, Debug)]
pub enum Icinga2ApiClientActorError {
    #[fail(display = "ServerNotAvailableError: cannot connect to [{}]", message)]
    ServerNotAvailableError { message: String },
}

pub struct Icinga2ApiClientActor {
    icinga2_ip: String,
    icinga2_port: u32,
    icinga2_user: String,
    icinga2_pass: String,
}

impl Actor for Icinga2ApiClientActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("Icinga2ApiClientActor started.");
    }

}

impl Icinga2ApiClientActor {
    pub fn start_new<IP: Into<String> + 'static, U: Into<String> + 'static, P: Into<String> + 'static>(
        icinga2_ip: IP,
        icinga2_port: u32,
        icinga2_user: U,
        icinga2_pass: P
    ) -> Addr<Self> {
        Icinga2ApiClientActor::create(move |ctx: &mut Context<Icinga2ApiClientActor>| {
            Icinga2ApiClientActor {
                icinga2_ip: icinga2_ip.into(),
                icinga2_port,
                icinga2_user: icinga2_user.into(),
                icinga2_pass: icinga2_pass.into(),
            }
        })
    }
}

impl Handler<Icinga2ApiClientMessage> for Icinga2ApiClientActor {
    type Result = Result<(), Icinga2ApiClientActorError>;

    fn handle(&mut self, msg: Icinga2ApiClientMessage, ctx: &mut Context<Self>) -> Self::Result {
        trace!("Icinga2ApiClientMessage - received new message");

        let auth = format!("{}:{}", self.icinga2_user, self.icinga2_pass);
        let header_value = format!("Basic {}", base64::encode(&auth));

        let url = "";
        let request_body = "";

        actix::spawn(
            ClientRequest::post(url)
                //.with_connector(connector)
                .header(header::ACCEPT, "application/json")
                .header(header::AUTHORIZATION, header_value)
                .timeout(Duration::from_secs(10))
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
        );

        Ok(())

    }
}
