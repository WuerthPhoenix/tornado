use tornado_collector_jmespath::JMESPathEventCollector;
use actix::Actor;
use actix::SyncContext;
use log::info;
use crate::config::Stream;
use crate::config::Icinga2ClientConfig;

pub struct Icinga2StreamActor {
    pub icinga_config: Icinga2ClientConfig,
    pub collector: JMESPathEventCollector,
    pub stream_config: Stream
}

impl Actor for Icinga2StreamActor {
    type Context = SyncContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!("Starting Icinga2StreamActor with stream config: {:#?}", self.stream_config);


    }
}