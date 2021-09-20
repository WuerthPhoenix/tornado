pub mod actors;
pub mod config;

use actix::dev::ToEnvelope;
use actix::prelude::*;
use log::*;
use std::io::{stdin, BufRead};
use std::thread;
use tornado_common::actors::message::{EventMessage, StringMessage};
use tornado_common::actors::nats_publisher::NatsPublisherActor;
use tornado_common::actors::tcp_client::TcpClientActor;
use tornado_common::actors::TornadoConnectionChannel;
use tornado_common::TornadoError;
use tornado_common_logger::elastic_apm::DEFAULT_APM_SERVER_CREDENTIALS_FILENAME;
use tornado_common_logger::setup_logger;

#[actix_rt::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let arg_matches = config::arg_matches();

    let config_dir = arg_matches.value_of("config-dir").expect("config-dir should be provided");

    let mut collector_config = config::build_config(config_dir)?;
    let apm_server_api_credentials_filepath =
        format!("{}/{}", config_dir, DEFAULT_APM_SERVER_CREDENTIALS_FILENAME);
    // Get the result and log the error later because the logger is not available yet
    let apm_credentials_read_result = collector_config
        .logger
        .tracing_elastic_apm
        .read_apm_server_api_credentials_if_not_set(&apm_server_api_credentials_filepath);

    // Setup logger
    let _guard = setup_logger(collector_config.logger)?;
    if let Err(apm_credentials_read_error) = apm_credentials_read_result {
        warn!(
            "Could not set APM Server credentials from file '{}'. Error: {:?}",
            apm_server_api_credentials_filepath, apm_credentials_read_error
        );
    }

    info!("Rsyslog collector started");

    let message_queue_size = collector_config.rsyslog_collector.message_queue_size;
    //
    // WARN:
    // This 'if' block contains some duplicated code to allow temporary compatibility with the config file format of the previous release.
    // It will be removed in the next release when the `tornado_connection_channel` will be mandatory.
    //
    if let (Some(tornado_event_socket_ip), Some(tornado_event_socket_port)) = (
        collector_config.rsyslog_collector.tornado_event_socket_ip,
        collector_config.rsyslog_collector.tornado_event_socket_port,
    ) {
        info!("Connect to Tornado through TCP socket");
        // Start TcpWriter
        let tornado_tcp_address =
            format!("{}:{}", tornado_event_socket_ip, tornado_event_socket_port,);

        let actor_address = TcpClientActor::start_new(tornado_tcp_address, message_queue_size);
        start(actor_address, message_queue_size);
    } else if let Some(connection_channel) =
        collector_config.rsyslog_collector.tornado_connection_channel
    {
        match connection_channel {
            TornadoConnectionChannel::Nats { nats } => {
                info!("Connect to Tornado through NATS");
                let actor_address = NatsPublisherActor::start_new(nats, message_queue_size).await?;
                start(actor_address, message_queue_size);
            }
            TornadoConnectionChannel::Tcp { tcp_socket_ip, tcp_socket_port } => {
                info!("Connect to Tornado through TCP socket");
                // Start TcpWriter
                let tornado_tcp_address = format!("{}:{}", tcp_socket_ip, tcp_socket_port,);

                let actor_address =
                    TcpClientActor::start_new(tornado_tcp_address, message_queue_size);
                start(actor_address, message_queue_size);
            }
        };
    } else {
        return Err(TornadoError::ConfigurationError {
            message: "A communication channel must be specified.".to_owned(),
        }
        .into());
    }

    tokio::signal::ctrl_c().await.unwrap();
    println!("Ctrl-C received, shutting down");
    System::current().stop();

    Ok(())
}

fn start<A: Actor + actix::Handler<EventMessage>>(actor_address: Addr<A>, message_queue_size: usize)
where
    <A as Actor>::Context: ToEnvelope<A, tornado_common::actors::message::EventMessage>,
{
    // Start Rsyslog collector
    let rsyslog_addr =
        actors::collector::RsyslogCollectorActor::start_new(actor_address, message_queue_size);

    let system = System::current();
    thread::spawn(move || {
        let stdin = stdin();
        let mut stdin_lock = stdin.lock();

        loop {
            let mut input = String::new();
            match stdin_lock.read_line(&mut input) {
                Ok(len) => {
                    if len == 0 {
                        info!("EOF received. Stopping Rsyslog collector.");
                        system.stop();
                    } else {
                        rsyslog_addr.try_send(StringMessage { msg: input }).unwrap_or_else(|err| error!("RsyslogCollector -  Error while sending message to RsyslogCollectorActor. Error: {}", err));
                    }
                }
                Err(error) => {
                    error!("error: {:?}", error);
                    system.stop();
                }
            }
        }
    });
}
