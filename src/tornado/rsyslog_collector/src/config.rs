use structopt::StructOpt;
use tornado_common_logger::LoggerConfig;

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct Io {
    /// Set the size of the in-memory queue where messages will be stored before being written
    /// to the output socket.
    #[structopt(long, default_value = "10000")]
    pub uds_mailbox_capacity: usize,

    /// The Tornado TCP address where outgoing events will be written
    #[structopt(long, default_value = "127.0.0.1:4747")]
    pub tornado_tcp_address: String,
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
