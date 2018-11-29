use tornado_common_logger::LoggerConfig;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct Io {
    /// Set the size of the in memory queue where messages are stored before being written
    /// to the output socket.
    #[structopt(long, default_value="10000")]
    pub uds_mailbox_capacity: usize,

    /// The Unix Socket path where to write the outcoming events.
    #[structopt(long, default_value="/tmp/tornado")]
    pub uds_path: String,
}

#[derive(Debug, StructOpt)]
pub struct Conf {
    // Configure the logger.
    #[structopt(flatten)]
    pub logger: LoggerConfig,

    // Configure the Input/Output options.
    #[structopt(flatten)]
    pub io: Io,
}

impl Conf {
    pub fn new() -> Self {
        Conf::from_args()
    }
}
