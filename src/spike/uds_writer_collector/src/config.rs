use tornado_common_logger::LoggerConfig;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct Io {
    /// The filesystem folder where the Events are saved in json format.
    #[structopt(long, default_value="./events")]
    pub json_events_path: String,

    /// The Unix Socket path where to write the events.
    #[structopt(long, default_value="/var/run/tornado/tornado.sock")]
    pub uds_path: String,

    /// How many times each event should be sent.
    #[structopt(long, default_value="1000")]
    pub repeat_send: usize,
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
