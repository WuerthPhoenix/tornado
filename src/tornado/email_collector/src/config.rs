use structopt::StructOpt;
use tornado_common_logger::LoggerConfig;

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct Io {
    /// Set the size of the in-memory queue where messages will be stored before being written
    /// to the output socket.
    #[structopt(long, default_value = "10000")]
    pub message_queue_size: usize,

    /// The Unix Socket path where we will listen for incoming emails.
    #[structopt(long, default_value = "/var/run/tornado/email.sock")]
    pub uds_path: String,

    /// The Tornado IP address where outgoing events will be written
    #[structopt(long, default_value = "127.0.0.1")]
    pub tornado_event_socket_ip: String,

    /// The Tornado port where outgoing events will be written
    #[structopt(long, default_value = "4747")]
    pub tornado_event_socket_port: u16,
}

#[derive(Debug, StructOpt)]
pub struct Conf {
    #[structopt(flatten)]
    pub logger: LoggerConfig,

    #[structopt(flatten)]
    pub io: Io,
}

impl Conf {
    pub fn build() -> Self {
        Conf::from_args()
    }
}
